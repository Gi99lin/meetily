"use client";
import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { Summary, SummaryResponse } from '@/types';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { TranscriptPanel } from '@/components/MeetingDetails/TranscriptPanel';
import { SummaryPanel } from '@/components/MeetingDetails/SummaryPanel';
import { ModelConfig } from '@/components/ModelSettingsModal';

// Custom hooks
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import { useTemplates } from '@/hooks/meeting-details/useTemplates';
import { useCopyOperations } from '@/hooks/meeting-details/useCopyOperations';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useConfig } from '@/contexts/ConfigContext';

const TRANSCRIPT_PANEL_WIDTH_KEY = 'meetily-transcript-panel-width';
const DEFAULT_TRANSCRIPT_PANEL_WIDTH = 420;
const MIN_TRANSCRIPT_PANEL_WIDTH = 280;
const MAX_TRANSCRIPT_PANEL_WIDTH = 800;

export default function PageContent({
  meeting,
  summaryData,
  shouldAutoGenerate = false,
  onAutoGenerateComplete,
  onMeetingUpdated,
  onRefetchTranscripts,
  // Pagination props for efficient transcript loading
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
}: {
  meeting: any;
  summaryData: Summary | null;
  shouldAutoGenerate?: boolean;
  onAutoGenerateComplete?: () => void;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  // Pagination props
  segments?: any[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
}) {
  console.log('📄 PAGE CONTENT: Initializing with data:', {
    meetingId: meeting.id,
    summaryDataKeys: summaryData ? Object.keys(summaryData) : null,
    transcriptsCount: meeting.transcripts?.length
  });

  // State
  const [customPrompt, setCustomPrompt] = useState<string>('');
  const [isRecording] = useState(false);
  const [summaryResponse] = useState<SummaryResponse | null>(null);

  // Resizable transcript panel width, persisted across sessions. A ref
  // tracks the live value during drag so the mousemove/mouseup handlers
  // (attached once per drag, not re-created on every render) always read
  // the latest width instead of a stale closure.
  const [transcriptPanelWidth, setTranscriptPanelWidth] = useState(DEFAULT_TRANSCRIPT_PANEL_WIDTH);
  const transcriptPanelWidthRef = useRef(DEFAULT_TRANSCRIPT_PANEL_WIDTH);

  useEffect(() => {
    const saved = localStorage.getItem(TRANSCRIPT_PANEL_WIDTH_KEY);
    if (!saved) return;
    const parsed = parseInt(saved, 10);
    if (Number.isNaN(parsed)) return;
    const clamped = Math.min(MAX_TRANSCRIPT_PANEL_WIDTH, Math.max(MIN_TRANSCRIPT_PANEL_WIDTH, parsed));
    setTranscriptPanelWidth(clamped);
    transcriptPanelWidthRef.current = clamped;
  }, []);

  const handlePanelResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = transcriptPanelWidthRef.current;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const delta = moveEvent.clientX - startX;
      const newWidth = Math.min(
        MAX_TRANSCRIPT_PANEL_WIDTH,
        Math.max(MIN_TRANSCRIPT_PANEL_WIDTH, startWidth + delta)
      );
      transcriptPanelWidthRef.current = newWidth;
      setTranscriptPanelWidth(newWidth);
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      localStorage.setItem(TRANSCRIPT_PANEL_WIDTH_KEY, String(transcriptPanelWidthRef.current));
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  // Ref to store the modal open function from SummaryGeneratorButtonGroup
  const openModelSettingsRef = useRef<(() => void) | null>(null);

  // Sidebar context
  const { serverAddress } = useSidebar();

  // Get model config from ConfigContext
  const { modelConfig, setModelConfig } = useConfig();

  // Custom hooks
  const meetingData = useMeetingData({ meeting, summaryData, onMeetingUpdated });
  const templates = useTemplates();

  // Callback to register the modal open function
  const handleRegisterModalOpen = (openFn: () => void) => {
    console.log('📝 Registering modal open function in PageContent');
    openModelSettingsRef.current = openFn;
  };

  // Callback to trigger modal open (called from error handler)
  const handleOpenModelSettings = () => {
    console.log('🔔 Opening model settings from PageContent');
    if (openModelSettingsRef.current) {
      openModelSettingsRef.current();
    } else {
      console.warn('⚠️ Modal open function not yet registered');
    }
  };

  // Save model config to backend database and sync via event
  const handleSaveModelConfig = async (config?: ModelConfig) => {
    if (!config) return;
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey ?? null,
        ollamaEndpoint: config.ollamaEndpoint ?? null,
      });

      // Emit event so ConfigContext and other listeners stay in sync
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      toast.success('Model settings saved successfully');
    } catch (error) {
      console.error('Failed to save model config:', error);
      toast.error('Failed to save model settings');
    }
  };

  const summaryGeneration = useSummaryGeneration({
    meeting,
    transcripts: meetingData.transcripts,
    modelConfig: modelConfig,
    isModelConfigLoading: false, // ConfigContext loads on mount
    selectedTemplate: templates.selectedTemplate,
    onMeetingUpdated,
    updateMeetingTitle: meetingData.updateMeetingTitle,
    setAiSummary: meetingData.setAiSummary,
    onOpenModelSettings: handleOpenModelSettings,
  });

  const copyOperations = useCopyOperations({
    meeting,
    transcripts: meetingData.transcripts,
    meetingTitle: meetingData.meetingTitle,
    aiSummary: meetingData.aiSummary,
    blockNoteSummaryRef: meetingData.blockNoteSummaryRef,
  });

  const meetingOperations = useMeetingOperations({
    meeting,
  });

  // Track page view
  useEffect(() => {
    Analytics.trackPageView('meeting_details');
  }, []);

  // Auto-generate summary when flag is set
  useEffect(() => {
    let cancelled = false;

    const autoGenerate = async () => {
      if (shouldAutoGenerate && meetingData.transcripts.length > 0 && !cancelled) {
        console.log(`🤖 Auto-generating summary with ${modelConfig.provider}/${modelConfig.model}...`);
        await summaryGeneration.handleGenerateSummary('');

        // Notify parent that auto-generation is complete (only if not cancelled)
        if (onAutoGenerateComplete && !cancelled) {
          onAutoGenerateComplete();
        }
      }
    };

    autoGenerate();

    // Cleanup: cancel if component unmounts or meeting changes
    return () => {
      cancelled = true;
    };
  }, [shouldAutoGenerate, meeting.id]); // Re-run if meeting changes

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      className="flex flex-col h-screen bg-gray-50"
    >
      <div className="flex flex-1 overflow-hidden">
        <div className="hidden md:flex flex-shrink-0 h-full" style={{ width: transcriptPanelWidth }}>
          <TranscriptPanel
            transcripts={meetingData.transcripts}
            customPrompt={customPrompt}
            onPromptChange={setCustomPrompt}
            onCopyTranscript={copyOperations.handleCopyTranscript}
            onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
            isRecording={isRecording}
            disableAutoScroll={true}
            // Pagination props for efficient loading
            usePagination={true}
            segments={segments}
            hasMore={hasMore}
            isLoadingMore={isLoadingMore}
            totalCount={totalCount}
            loadedCount={loadedCount}
            onLoadMore={onLoadMore}
            // Retranscription props
            meetingId={meeting.id}
            meetingFolderPath={meeting.folder_path}
            onRefetchTranscripts={onRefetchTranscripts}
          />
        </div>
        <div
          onMouseDown={handlePanelResizeStart}
          className="hidden md:block w-1 flex-shrink-0 cursor-col-resize bg-gray-200 hover:bg-blue-400 active:bg-blue-500 transition-colors"
          title="Drag to resize"
        />
        <SummaryPanel
          meeting={meeting}
          meetingTitle={meetingData.meetingTitle}
          onTitleChange={meetingData.handleTitleChange}
          isEditingTitle={meetingData.isEditingTitle}
          onStartEditTitle={() => meetingData.setIsEditingTitle(true)}
          onFinishEditTitle={() => meetingData.setIsEditingTitle(false)}
          isTitleDirty={meetingData.isTitleDirty}
          summaryRef={meetingData.blockNoteSummaryRef}
          isSaving={meetingData.isSaving}
          onSaveAll={meetingData.saveAllChanges}
          onCopySummary={copyOperations.handleCopySummary}
          onOpenFolder={meetingOperations.handleOpenMeetingFolder}
          aiSummary={meetingData.aiSummary}
          summaryStatus={summaryGeneration.summaryStatus}
          transcripts={meetingData.transcripts}
          modelConfig={modelConfig}
          setModelConfig={setModelConfig}
          onSaveModelConfig={handleSaveModelConfig}
          onGenerateSummary={summaryGeneration.handleGenerateSummary}
          onStopGeneration={summaryGeneration.handleStopGeneration}
          customPrompt={customPrompt}
          summaryResponse={summaryResponse}
          onSaveSummary={meetingData.handleSaveSummary}
          onSummaryChange={meetingData.handleSummaryChange}
          onDirtyChange={meetingData.setIsSummaryDirty}
          summaryError={summaryGeneration.summaryError}
          onRegenerateSummary={summaryGeneration.handleRegenerateSummary}
          getSummaryStatusMessage={summaryGeneration.getSummaryStatusMessage}
          availableTemplates={templates.availableTemplates}
          selectedTemplate={templates.selectedTemplate}
          onTemplateSelect={templates.handleTemplateSelection}
          isModelConfigLoading={false}
          onOpenModelSettings={handleRegisterModalOpen}
        />
      </div>
    </motion.div>
  );
}
