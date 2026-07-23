"use client";

import { useState, useCallback, useEffect, useRef } from 'react';
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
  const [hasResults, setHasResults] = useState(false);

  // Text labels are hidden below a pixel threshold measured on this row's
  // own container, not Tailwind's viewport breakpoints — the transcript
  // panel is now user-resizable, so a `lg:` class would show labels based
  // on the whole window's width regardless of how narrow this panel
  // actually is, clipping the button row exactly as reported.
  const containerRef = useRef<HTMLDivElement>(null);
  const [showLabels, setShowLabels] = useState(true);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? 0;
      setShowLabels(width > 380);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Check once whether this meeting already has ML speaker labels, so a
  // click on "Detect Speakers" can confirm before re-running (it re-decodes
  // the whole recording and can take several minutes).
  useEffect(() => {
    if (!meetingId) return;
    let cancelled = false;
    diarizationService
      .hasResults(meetingId)
      .then((result) => {
        if (!cancelled) setHasResults(result);
      })
      .catch(() => {
        // Transient failure; leave hasResults as-is (defaults to false).
      });
    return () => {
      cancelled = true;
    };
  }, [meetingId]);

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
                setHasResults(true);
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

    if (hasResults) {
      const confirmed = window.confirm(
        'Speakers were already detected for this recording. Re-run detection? ' +
        'This re-processes the whole recording and can take several minutes.'
      );
      if (!confirmed) return;
    }

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
      setHasResults(true);

      await onRefetchTranscripts?.();
      await onRefetchSpeakerNames?.();
    } catch (error) {
      toast.error('Speaker detection failed', {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsDiarizing(false);
    }
  }, [meetingId, isDiarizing, hasResults, onRefetchTranscripts, onRefetchSpeakerNames]);

  return (
    <div ref={containerRef} className="flex items-center justify-center w-full gap-2">
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
          <Copy className={showLabels ? 'mr-2' : ''} size={18} />
          {showLabels && <span>Copy</span>}
        </Button>

        <Button
          size="sm"
          variant="outline"
          onClick={() => {
            Analytics.trackButtonClick('open_recording_folder', 'meeting_details');
            onOpenMeetingFolder();
          }}
          title="Open Recording Folder"
        >
          <FolderOpen className={showLabels ? 'mr-2' : ''} size={18} />
          {showLabels && <span>Recording</span>}
        </Button>

        {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
          <Button
            size="sm"
            variant="outline"
            className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200"
            onClick={() => {
              Analytics.trackButtonClick('enhance_transcript', 'meeting_details');
              setShowRetranscribeDialog(true);
            }}
            title="Retranscribe to enhance your recorded audio"
          >
            <RefreshCw className={showLabels ? 'mr-2' : ''} size={18} />
            {showLabels && <span>Enhance</span>}
          </Button>
        )}

        {betaFeatures.speakerDiarization && meetingId && (
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              Analytics.trackButtonClick('detect_speakers', 'meeting_details');
              handleDetectSpeakers();
            }}
            disabled={isDiarizing || transcriptCount === 0}
            title={hasResults ? 'Re-run speaker detection' : 'Detect distinct speakers in this recording'}
          >
            {isDiarizing ? (
              <Loader2 className={`animate-spin ${showLabels ? 'mr-2' : ''}`} size={18} />
            ) : (
              <Users className={showLabels ? 'mr-2' : ''} size={18} />
            )}
            {showLabels && (
              <span>{isDiarizing ? 'Detecting...' : hasResults ? 'Re-detect Speakers' : 'Detect Speakers'}</span>
            )}
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
