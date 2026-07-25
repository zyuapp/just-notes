import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import { formatDuration } from "../lib/format";
import { segmentClasses, type SpeakerRunEdges } from "../lib/transcript";

type SegmentBlockProps = {
  segment: TranscriptSegment;
  runEdges: SpeakerRunEdges;
  active: boolean;
};

export function SegmentBlock({ segment, runEdges, active }: SegmentBlockProps) {
  return (
    <article className={segmentClasses(segment, runEdges, active)}>
      {runEdges.isStart && (
        <header className="segment-head">
          <strong>{segment.speaker}</strong>
          <time>{formatDuration(segment.startMs)}</time>
        </header>
      )}
      <p>{segment.text}</p>
    </article>
  );
}
