import { useEffect, useRef } from "react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import { mergeLiveSegments, speakerRunEdges, visibleSegments } from "../lib/transcript";
import { SegmentBlock } from "./SegmentBlock";
import { TranscriptEmptyState } from "./TranscriptEmptyState";

type TranscriptSurfaceProps = {
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  liveSegments: TranscriptSegment[];
  transcriptionStatus: TranscriptionStatusPayload | null;
  query: string;
};

export function TranscriptSurface({
  recorderState,
  selectedThread,
  liveSegments,
  transcriptionStatus,
  query,
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
              runEdges={speakerRunEdges(items, position)}
              active={isRecording && position === items.length - 1}
            />
          ))
        )}
      </div>
    </section>
  );
}
