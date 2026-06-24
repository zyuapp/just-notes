import type { TranscriptSegment } from "../bindings/TranscriptSegment";

export type IndexedSegment = { segment: TranscriptSegment; index: number };

export function visibleSegments(segments: TranscriptSegment[], query: string): IndexedSegment[] {
  const indexed = segments.map((segment, index) => ({ segment, index }));
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return indexed;
  return indexed.filter(({ segment }) => segment.text.toLowerCase().includes(trimmed));
}

export function showsSpeakerHeader(items: IndexedSegment[], position: number): boolean {
  if (position === 0) return true;
  return items[position - 1].segment.speaker !== items[position].segment.speaker;
}

export function displaySpeaker(speaker: string, labels: Record<string, string>): string {
  return labels[speaker] ?? speaker;
}

export function transcriptToText(
  segments: TranscriptSegment[],
  labels: Record<string, string>,
  formatTime: (ms: number) => string,
): string {
  return segments
    .map(
      (segment) =>
        `[${formatTime(segment.startMs)}] ${displaySpeaker(segment.speaker, labels)}: ${segment.text}`,
    )
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
