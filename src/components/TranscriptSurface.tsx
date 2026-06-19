import { useEffect, useRef } from "react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import { displaySpeaker, showsSpeakerHeader, visibleSegments } from "../lib/transcript";
import { SegmentBlock } from "./SegmentBlock";
import { TranscriptEmptyState } from "./TranscriptEmptyState";

type TranscriptSurfaceProps = {
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  query: string;
  onCreateThread: () => void;
  onStartModelDownload: () => void;
  onStartRecording: () => void;
  onSaveSegmentText: (index: number, text: string) => void;
};

export function TranscriptSurface({
  recorderState,
  selectedThread,
  transcriptionStatus,
  query,
  onCreateThread,
  onStartModelDownload,
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
        <TranscriptEmptyState
          hasThread={selectedThread !== null}
          canStart={recorderState === "idle"}
          transcriptionStatus={transcriptionStatus}
          onCreateThread={onCreateThread}
          onStartModelDownload={onStartModelDownload}
          onStartRecording={onStartRecording}
        />
      </section>
    );
  }

  const items = visibleSegments(selectedThread.segments, query);
  const editable = recorderState === "idle" && selectedThread.summary.status === "idle";

  return (
    <section className="transcript-surface" ref={surfaceRef}>
      <div className="transcript-measure">
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
      </div>
    </section>
  );
}
