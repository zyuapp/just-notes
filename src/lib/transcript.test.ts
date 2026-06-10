import { describe, expect, test } from "bun:test";
import type { LiveTranscriptSegmentPayload } from "../bindings/LiveTranscriptSegmentPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import {
  applyLiveSegmentToThread,
  applyLiveSegmentToThreadList,
  displaySpeaker,
  showsSpeakerHeader,
  sortTranscriptSegments,
  visibleSegments,
} from "./transcript";

const baseSummary: ThreadSummary = {
  id: "thread-1",
  title: "Thread",
  createdAtMs: 100,
  updatedAtMs: 100,
  segmentCount: 1,
  status: "idle",
  durationMs: 0,
  snippet: "",
  hasAudio: false,
  path: "/threads/thread-1",
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
      speakerLabels: {},
      transcriptMarkdownPath: "/tmp/transcript.md",
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
      speakerLabels: {},
      transcriptMarkdownPath: "/tmp/transcript.md",
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

describe("visibleSegments", () => {
  test("keeps original indices when filtering by query", () => {
    const segments = [
      { ...segment("mic", 1_000, 2_000), text: "hello world" },
      { ...segment("system", 3_000, 4_000), text: "different topic" },
      { ...segment("mic", 5_000, 6_000), text: "world again" },
    ];

    const matches = visibleSegments(segments, "WORLD");

    expect(matches.map((item) => item.index)).toEqual([0, 2]);
    expect(visibleSegments(segments, "  ")).toHaveLength(3);
  });
});

describe("showsSpeakerHeader", () => {
  test("shows a header only when the speaker changes", () => {
    const items = visibleSegments(
      [segment("mic", 1_000, 2_000), segment("mic", 3_000, 4_000), segment("system", 5_000, 6_000)],
      "",
    );

    expect(showsSpeakerHeader(items, 0)).toBe(true);
    expect(showsSpeakerHeader(items, 1)).toBe(false);
    expect(showsSpeakerHeader(items, 2)).toBe(true);
  });
});

describe("displaySpeaker", () => {
  test("prefers the configured label", () => {
    expect(displaySpeaker("You", { You: "Zhuocheng" })).toBe("Zhuocheng");
    expect(displaySpeaker("Others", {})).toBe("Others");
  });
});
