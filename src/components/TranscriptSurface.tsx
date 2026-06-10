import { Circle, FilePlus2, Waves } from "lucide-react";
import { useEffect, useRef } from "react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { RecorderState } from "../features/app/state";
import { displaySpeaker, showsSpeakerHeader, visibleSegments } from "../lib/transcript";
import { SegmentBlock } from "./SegmentBlock";

type TranscriptSurfaceProps = {
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  query: string;
  onCreateThread: () => void;
  onStartRecording: () => void;
  onSaveSegmentText: (index: number, text: string) => void;
};

export function TranscriptSurface({
  recorderState,
  selectedThread,
  query,
  onCreateThread,
  onStartRecording,
  onSaveSegmentText,
}: TranscriptSurfaceProps) {
  const surfaceRef = useRef<HTMLElement | null>(null);
  const isRecording = recorderState === "recording";
  const segmentCount = selectedThread?.segments.length ?? 0;

  useEffect(() => {
    const surface = surfaceRef.current;
    if (surface && isRecording) {
      surface.scrollTop = surface.scrollHeight;
    }
  }, [isRecording, segmentCount]);

  if (!selectedThread || segmentCount === 0) {
    return (
      <section className="transcript-surface" ref={surfaceRef}>
        <EmptyState
          hasThread={selectedThread !== null}
          canStart={recorderState === "idle"}
          onCreateThread={onCreateThread}
          onStartRecording={onStartRecording}
        />
      </section>
    );
  }

  const items = visibleSegments(selectedThread.segments, query);
  const editable = !isRecording && recorderState === "idle";

  return (
    <section className="transcript-surface" ref={surfaceRef}>
      {items.length === 0 ? (
        <p className="transcript-no-match">No transcript text matches “{query.trim()}”.</p>
      ) : (
        items.map((item, position) => (
          <SegmentBlock
            key={`${item.segment.source}-${item.segment.startMs}-${item.index}`}
            segment={item.segment}
            index={item.index}
            showHeader={showsSpeakerHeader(items, position)}
            speakerLabel={displaySpeaker(item.segment.speaker, selectedThread.speakerLabels)}
            active={isRecording && position === items.length - 1}
            editable={editable}
            onSaveText={onSaveSegmentText}
          />
        ))
      )}
    </section>
  );
}

type EmptyStateProps = {
  hasThread: boolean;
  canStart: boolean;
  onCreateThread: () => void;
  onStartRecording: () => void;
};

function EmptyState({ hasThread, canStart, onCreateThread, onStartRecording }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <div className="empty-mark">
        <Waves size={28} aria-hidden="true" />
      </div>
      <div>
        <h2>{hasThread ? "Ready when you are" : "Ready to capture"}</h2>
        <p>
          {hasThread
            ? "Start recording to add the first transcript segment to this thread."
            : "Record a conversation now, or create a blank thread for notes you will fill in later."}
        </p>
      </div>
      <div className="empty-actions">
        <button type="button" className="empty-primary" onClick={onStartRecording} disabled={!canStart}>
          <Circle size={15} aria-hidden="true" />
          <span>Start recording</span>
        </button>
        {!hasThread && (
          <button type="button" className="empty-secondary" onClick={onCreateThread}>
            <FilePlus2 size={15} aria-hidden="true" />
            <span>New blank thread</span>
          </button>
        )}
      </div>
    </div>
  );
}
