import { describe, expect, test } from "bun:test";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import {
  mergeLiveSegments,
  segmentClasses,
  sortTranscriptSegments,
  speakerRunEdges,
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

describe("speakerRunEdges", () => {
  test("marks the first and last segment of each same-speaker run", () => {
    const items = visibleSegments(
      [
        segment("mic", 1_000, 2_000),
        segment("mic", 3_000, 4_000),
        segment("mic", 5_000, 6_000),
        segment("system", 7_000, 8_000),
      ],
      "",
    );

    expect(items.map((_, position) => speakerRunEdges(items, position))).toEqual([
      { isStart: true, isEnd: false },
      { isStart: false, isEnd: false },
      { isStart: false, isEnd: true },
      { isStart: true, isEnd: true },
    ]);
  });

  test("breaks the run where a query hid the segments in between", () => {
    const segments = [
      { ...segment("mic", 1_000, 2_000), text: "keep me" },
      { ...segment("mic", 3_000, 4_000), text: "keep me too" },
      { ...segment("system", 5_000, 6_000), text: "filtered out" },
      { ...segment("mic", 7_000, 8_000), text: "keep me last" },
    ];

    const items = visibleSegments(segments, "keep me");

    expect(items.map((item) => item.index)).toEqual([0, 1, 3]);
    expect(items.map((_, position) => speakerRunEdges(items, position))).toEqual([
      { isStart: true, isEnd: false },
      { isStart: false, isEnd: true },
      { isStart: true, isEnd: true },
    ]);
  });

  test("groups by speaker, so one channel can hold several runs", () => {
    const items = visibleSegments(
      [
        { ...segment("system", 1_000, 2_000), speaker: "Speaker 1" },
        { ...segment("system", 3_000, 4_000), speaker: "Speaker 2" },
      ],
      "",
    );

    expect(items.map((_, position) => speakerRunEdges(items, position))).toEqual([
      { isStart: true, isEnd: true },
      { isStart: true, isEnd: true },
    ]);
  });
});

describe("segmentClasses", () => {
  const bothEdges = { isStart: true, isEnd: true };
  const mic = segment("mic", 0, 1_000);

  test("keys the you channel on source, not on the speaker label", () => {
    expect(segmentClasses(mic, bothEdges, false)).toBe("segment you run-end");
    expect(segmentClasses(segment("system", 0, 1_000), bothEdges, false)).toBe("segment run-end");
    expect(segmentClasses({ ...mic, speaker: "Renamed" }, bothEdges, false)).toBe(
      "segment you run-end",
    );
  });

  test("derives continuation and run-end from the run edges", () => {
    expect(segmentClasses(mic, { isStart: true, isEnd: false }, false)).toBe("segment you");
    expect(segmentClasses(mic, { isStart: false, isEnd: false }, false)).toBe(
      "segment you continuation",
    );
    expect(segmentClasses(mic, { isStart: false, isEnd: true }, false)).toBe(
      "segment you continuation run-end",
    );
  });

  test("adds the active class while recording", () => {
    expect(segmentClasses(mic, bothEdges, true)).toBe("segment you run-end active");
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
