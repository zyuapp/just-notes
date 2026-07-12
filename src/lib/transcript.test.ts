import { describe, expect, test } from "bun:test";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import {
  mergeLiveSegments,
  showsSpeakerHeader,
  sortTranscriptSegments,
  visibleSegments,
} from "./transcript";

function segment(source: string, startMs: number, endMs: number): TranscriptSegment {
  return {
    source,
    speaker: source === "mic" ? "You" : "Others",
    startMs,
    endMs,
    text: `${source} ${startMs}`,
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

describe("mergeLiveSegments", () => {
  test("drops live segments already present in base and sorts the result", () => {
    const base = [segment("mic", 1_000, 2_000), segment("system", 3_000, 4_000)];
    const live = [segment("mic", 1_000, 2_000), segment("mic", 5_000, 6_000)];

    const merged = mergeLiveSegments(base, live);

    expect(merged.map((item) => `${item.startMs}:${item.source}`)).toEqual([
      "1000:mic",
      "3000:system",
      "5000:mic",
    ]);
  });

  test("keeps a live segment that differs only by endMs or source", () => {
    const merged = mergeLiveSegments(
      [segment("mic", 1_000, 2_000)],
      [segment("mic", 1_000, 2_500), segment("system", 1_000, 2_000)],
    );

    expect(merged).toHaveLength(3);
  });

  test("returns base unchanged when live is empty", () => {
    const base = [segment("mic", 1_000, 2_000)];
    expect(mergeLiveSegments(base, [])).toBe(base);
  });
});
