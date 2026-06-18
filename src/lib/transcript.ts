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
