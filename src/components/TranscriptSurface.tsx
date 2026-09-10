import { useEffect, useRef } from "react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import { type IndexedSegment, mergeLiveSegments, speakerRunEdges, visibleSegments } from "../lib/transcript";
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
  // Keep the existing speaker/search boundaries; only join their visual layout.
  const runs: IndexedSegment[][] = [];
  items.forEach((item, position) => {
    if (speakerRunEdges(items, position).isStart) runs.push([]);
    runs[runs.length - 1].push(item);
  });

  return (
    <section className="transcript-surface" ref={surfaceRef}>
      <div className="transcript-measure">
        {items.length === 0 ? (
          <p className="transcript-no-match">No transcript text matches “{query.trim()}”.</p>
        ) : (
          runs.map((run, position) => (
            <SegmentBlock
              key={`${run[0].segment.source}-${run[0].segment.startMs}-${run[0].index}`}
              items={run}
              query={query}
              active={isRecording && position === runs.length - 1}
            />
          ))
        )}
      </div>
    </section>
  );
}
