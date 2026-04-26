import type { LiveTranscriptSegmentPayload } from "../bindings/LiveTranscriptSegmentPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";

export function sortTranscriptSegments(segments: TranscriptSegment[]) {
  return [...segments].sort((left, right) => {
    if (left.startMs !== right.startMs) return left.startMs - right.startMs;
    return left.source.localeCompare(right.source);
  });
}

export function applyLiveSegmentToThread(
  thread: ThreadDetail | null,
  payload: LiveTranscriptSegmentPayload,
  updatedAtMs: number,
) {
  if (!thread || thread.summary.id !== payload.threadId) {
    return thread;
  }

  const segments = sortTranscriptSegments([...thread.segments, payload.segment]);
  return {
    ...thread,
    segments,
    summary: updateThreadSummaryForLiveSegment(thread.summary, updatedAtMs, segments.length),
  };
}

export function applyLiveSegmentToThreadList(
  threads: ThreadSummary[],
  payload: LiveTranscriptSegmentPayload,
  updatedAtMs: number,
) {
  return threads.map((thread) =>
    thread.id === payload.threadId
      ? updateThreadSummaryForLiveSegment(thread, updatedAtMs, thread.segmentCount + 1)
      : thread,
  );
}

function updateThreadSummaryForLiveSegment(
  summary: ThreadSummary,
  updatedAtMs: number,
  segmentCount: number,
) {
  return {
    ...summary,
    segmentCount,
    updatedAtMs,
  };
}
