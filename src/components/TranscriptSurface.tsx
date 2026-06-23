import { useEffect, useRef } from "react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import {
  displaySpeaker,
  mergeLiveSegments,
  showsSpeakerHeader,
  visibleSegments,
} from "../lib/transcript";
import { SegmentBlock } from "./SegmentBlock";
import { TranscriptEmptyState } from "./TranscriptEmptyState";

type TranscriptSurfaceProps = {
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  liveSegments: TranscriptSegment[];
  transcriptionStatus: TranscriptionStatusPayload | null;
  query: string;
  onSaveSegmentText: (index: number, text: string) => void;
};

export function TranscriptSurface({
  recorderState,
  selectedThread,
  liveSegments,
  transcriptionStatus,
  query,
  onSaveSegmentText,
}: TranscriptSurfaceProps) {
  const surfaceRef = useRef<HTMLElement | null>(null);
  const isRecording = recorderState === "recording";
  const baseSegments = selectedThread?.segments ?? [];
  const segments = mergeLiveSegments(baseSegments, liveSegments);
  const segmentCount = segments.length;

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
          recorderState={recorderState}
          transcriptionStatus={transcriptionStatus}
        />
      </section>
    );
  }

  const items = visibleSegments(segments, query);
  // Live preview segments are not yet persisted, so an index here would not map
  // to a stored segment; editing stays off until the finalized transcript lands.
  const editable =
    recorderState === "idle" &&
    selectedThread.summary.status === "idle" &&
    liveSegments.length === 0;

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
