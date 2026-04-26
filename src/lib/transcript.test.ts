import { describe, expect, test } from "bun:test";
import type { LiveTranscriptSegmentPayload } from "../bindings/LiveTranscriptSegmentPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import {
  applyLiveSegmentToThread,
  applyLiveSegmentToThreadList,
  sortTranscriptSegments,
} from "./transcript";

const baseSummary: ThreadSummary = {
  id: "thread-1",
  title: "Thread",
  createdAtMs: 100,
  updatedAtMs: 100,
  segmentCount: 1,
  status: "idle",
};

function segment(source: string, startMs: number, endMs: number): TranscriptSegment {
  return {
    source,
    speaker: source === "mic" ? "You" : "Others",
    startMs,
    endMs,
    text: `${source} ${startMs}`,
  };
}

function livePayload(threadId = "thread-1"): LiveTranscriptSegmentPayload {
  return {
    threadId,
    committedUntilMs: 12_000,
    segment: segment("mic", 3_000, 12_000),
  };
}

describe("sortTranscriptSegments", () => {
  test("sorts by start time then source", () => {
    const sorted = sortTranscriptSegments([
      segment("system", 5_000, 7_000),
      segment("system", 3_000, 4_000),
      segment("mic", 3_000, 4_000),
    ]);

    expect(sorted.map((item) => `${item.startMs}:${item.source}`)).toEqual([
      "3000:mic",
      "3000:system",
      "5000:system",
    ]);
  });
});

describe("applyLiveSegmentToThread", () => {
  test("adds matching live segments and updates the summary", () => {
    const thread: ThreadDetail = {
      summary: baseSummary,
      segments: [segment("system", 4_000, 8_000)],
      transcriptMarkdownPath: "/tmp/transcript.md",
      transcriptJsonlPath: "/tmp/transcript.jsonl",
    };

    const updated = applyLiveSegmentToThread(thread, livePayload(), 200);

    expect(updated?.segments.map((item) => `${item.startMs}:${item.source}`)).toEqual([
      "3000:mic",
      "4000:system",
    ]);
    expect(updated?.summary.segmentCount).toBe(2);
    expect(updated?.summary.updatedAtMs).toBe(200);
  });

  test("ignores payloads for other threads", () => {
    const thread: ThreadDetail = {
      summary: baseSummary,
      segments: [],
      transcriptMarkdownPath: "/tmp/transcript.md",
      transcriptJsonlPath: "/tmp/transcript.jsonl",
    };

    expect(applyLiveSegmentToThread(thread, livePayload("other"), 200)).toBe(thread);
  });
});

describe("applyLiveSegmentToThreadList", () => {
  test("increments only the matching thread", () => {
    const other = { ...baseSummary, id: "thread-2" };
    const updated = applyLiveSegmentToThreadList([baseSummary, other], livePayload(), 200);

    expect(updated[0].segmentCount).toBe(2);
    expect(updated[0].updatedAtMs).toBe(200);
    expect(updated[1]).toBe(other);
  });
});
