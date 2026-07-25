import type { TranscriptSegment } from "../bindings/TranscriptSegment";

export type IndexedSegment = { segment: TranscriptSegment; index: number };

export function visibleSegments(segments: TranscriptSegment[], query: string): IndexedSegment[] {
  const indexed = segments.map((segment, index) => ({ segment, index }));
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return indexed;
  return indexed.filter(({ segment }) => segment.text.toLowerCase().includes(trimmed));
}

export type SpeakerRunEdges = { isStart: boolean; isEnd: boolean };

// Two items continue one run only when they are neighbours in the unfiltered
// transcript, so a search that hides the segments between them cannot fuse them.
const continuesRun = (earlier: IndexedSegment, later: IndexedSegment) =>
  later.index === earlier.index + 1 && later.segment.speaker === earlier.segment.speaker;

// A run is a maximal group of adjacent segments sharing one speaker. `isStart` also
// decides header visibility; both edges let the speaker rail span the run as one line.
export function speakerRunEdges(items: IndexedSegment[], position: number): SpeakerRunEdges {
  const current = items[position];
  const previous = items[position - 1];
  const next = items[position + 1];
  return {
    isStart: !previous || !continuesRun(previous, current),
    isEnd: !next || !continuesRun(current, next),
  };
}

// The rail keys on `source` (the local mic channel) while runs key on `speaker`.
export function segmentClasses(
  segment: TranscriptSegment,
  runEdges: SpeakerRunEdges,
  active: boolean,
): string {
  const classes = ["segment"];
  if (segment.source === "mic") classes.push("you");
  if (!runEdges.isStart) classes.push("continuation");
  if (runEdges.isEnd) classes.push("run-end");
  if (active) classes.push("active");
  return classes.join(" ");
}

export function transcriptToText(
  segments: TranscriptSegment[],
  formatTime: (ms: number) => string,
): string {
  return segments
    .map((segment) => `[${formatTime(segment.startMs)}] ${segment.speaker}: ${segment.text}`)
    .join("\n");
}

export function sortTranscriptSegments(segments: TranscriptSegment[]) {
  return [...segments].sort((left, right) => {
    if (left.startMs !== right.startMs) return left.startMs - right.startMs;
    return left.source.localeCompare(right.source);
  });
}

const liveKey = (segment: TranscriptSegment) =>
  `${segment.source}:${segment.startMs}:${segment.endMs}`;

// Merges live preview segments onto the persisted base. Live segments are now
// persisted, so a mid-recording reload puts the same segment in both base and
// live; drop those duplicates and sort the result by time.
export function mergeLiveSegments(
  base: TranscriptSegment[],
  live: TranscriptSegment[],
): TranscriptSegment[] {
  if (live.length === 0) return base;
  const seen = new Set(base.map(liveKey));
  const fresh = live.filter((segment) => !seen.has(liveKey(segment)));
  if (fresh.length === 0) return base;
  return sortTranscriptSegments([...base, ...fresh]);
}
