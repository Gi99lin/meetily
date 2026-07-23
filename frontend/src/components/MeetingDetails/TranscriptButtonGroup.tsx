"use client";

import { useState, useCallback, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, FolderOpen, RefreshCw, Users, Loader2 } from 'lucide-react';
import { toast } from 'sonner';
import Analytics from '@/lib/analytics';
import { RetranscribeDialog } from './RetranscribeDialog';
import { useConfig } from '@/contexts/ConfigContext';
import { diarizationService } from '@/services/diarizationService';
import { areDiarizationModelsReady } from '@/types/diarization';


interface TranscriptButtonGroupProps {
  transcriptCount: number;
  onCopyTranscript: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
  onRefetchSpeakerNames?: () => Promise<void>;
}


export function TranscriptButtonGroup({
  transcriptCount,
  onCopyTranscript,
  onOpenMeetingFolder,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
  onRefetchSpeakerNames,
}: TranscriptButtonGroupProps) {
  const { betaFeatures } = useConfig();
  const [showRetranscribeDialog, setShowRetranscribeDialog] = useState(false);
  const [isDiarizing, setIsDiarizing] = useState(false);

  // isDiarizing is local state, so it resets on unmount even though the
  // backend task keeps running (e.g. navigating to Settings and back).
  // On mount, check whether this meeting actually has a run in progress;
  // if so, keep polling until it finishes, then refetch to pick up the
  // results — the original invocation's own refetch may have been lost
  // to that same unmount.
  useEffect(() => {
    if (!meetingId) return;
    let cancelled = false;
    let intervalId: ReturnType<typeof setInterval> | null = null;

    const checkStatus = async (): Promise<boolean> => {
      const running = await diarizationService.isDiarizationRunning(meetingId);
      if (!cancelled) setIsDiarizing(running);
      return running;
    };

    (async () => {
      try {
        const running = await checkStatus();
        if (!running || cancelled) return;
        intervalId = setInterval(async () => {
          try {
            const stillRunning = await checkStatus();
            if (!stillRunning && intervalId) {
              clearInterval(intervalId);
              intervalId = null;
              if (!cancelled) {
                await onRefetchTranscripts?.();
                await onRefetchSpeakerNames?.();
              }
            }
          } catch {
            // Transient status-check failure; retry on the next tick.
          }
        }, 3000);
      } catch {
        // Transient failure on the initial check; assume not running.
      }
    })();

    return () => {
      cancelled = true;
      if (intervalId) clearInterval(intervalId);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [meetingId]);

  const handleRetranscribeComplete = useCallback(async () => {
    // Refetch transcripts to show the updated data
    if (onRefetchTranscripts) {
      await onRefetchTranscripts();
    }
  }, [onRefetchTranscripts]);

  const handleDetectSpeakers = useCallback(async () => {
    if (!meetingId || isDiarizing) return;

    setIsDiarizing(true);
    try {
      const status = await diarizationService.getModelsStatus();
      if (!areDiarizationModelsReady(status)) {
        toast.info('Downloading speaker detection models (one-time, ~55MB)...');
        await diarizationService.downloadModels();
      }

      const result = await diarizationService.runDiarization(meetingId);
      toast.success(
        `Found ${result.num_speakers} speaker(s), labeled ${result.segments_updated} segment(s).`
      );

      await onRefetchTranscripts?.();
      await onRefetchSpeakerNames?.();
    } catch (error) {
      toast.error('Speaker detection failed', {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsDiarizing(false);
    }
  }, [meetingId, isDiarizing, onRefetchTranscripts, onRefetchSpeakerNames]);

  return (
    <div className="flex items-center justify-center w-full gap-2">
      <ButtonGroup>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            Analytics.trackButtonClick('copy_transcript', 'meeting_details');
            onCopyTranscript();
          }}
          disabled={transcriptCount === 0}
          title={transcriptCount === 0 ? 'No transcript available' : 'Copy Transcript'}
        >
          <Copy />
          <span className="hidden lg:inline">Copy</span>
        </Button>

        <Button
          size="sm"
          variant="outline"
          className="xl:px-4"
          onClick={() => {
            Analytics.trackButtonClick('open_recording_folder', 'meeting_details');
            onOpenMeetingFolder();
          }}
          title="Open Recording Folder"
        >
          <FolderOpen className="xl:mr-2" size={18} />
          <span className="hidden lg:inline">Recording</span>
        </Button>

        {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
          <Button
            size="sm"
            variant="outline"
            className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200 xl:px-4"
            onClick={() => {
              Analytics.trackButtonClick('enhance_transcript', 'meeting_details');
              setShowRetranscribeDialog(true);
            }}
            title="Retranscribe to enhance your recorded audio"
          >
            <RefreshCw className="xl:mr-2" size={18} />
            <span className="hidden lg:inline">Enhance</span>
          </Button>
        )}

        {betaFeatures.speakerDiarization && meetingId && (
          <Button
            size="sm"
            variant="outline"
            className="xl:px-4"
            onClick={() => {
              Analytics.trackButtonClick('detect_speakers', 'meeting_details');
              handleDetectSpeakers();
            }}
            disabled={isDiarizing || transcriptCount === 0}
            title="Detect distinct speakers in this recording"
          >
            {isDiarizing ? (
              <Loader2 className="xl:mr-2 animate-spin" size={18} />
            ) : (
              <Users className="xl:mr-2" size={18} />
            )}
            <span className="hidden lg:inline">{isDiarizing ? 'Detecting...' : 'Speakers'}</span>
          </Button>
        )}
      </ButtonGroup>

      {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
        <RetranscribeDialog
          open={showRetranscribeDialog}
          onOpenChange={setShowRetranscribeDialog}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onComplete={handleRetranscribeComplete}
        />
      )}
    </div>
  );
}
