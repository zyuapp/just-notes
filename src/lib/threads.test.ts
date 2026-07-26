import { describe, expect, test } from "bun:test";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { groupThreadsByDay, neighborThreadId } from "./threads";

function thread(id: string, updatedAtMs: number): ThreadSummary {
  return {
    id,
    title: id,
    createdAtMs: updatedAtMs,
    updatedAtMs,
    status: "idle",
    segmentCount: 0,
    durationMs: 0,
    snippet: "",
    hasAudio: false,
    path: `/threads/${id}`,
    calendar: null,
  };
}

describe("groupThreadsByDay", () => {
  test("labels today, yesterday, and older days", () => {
    const now = new Date(2026, 5, 9, 15, 0, 0);
    const today = new Date(2026, 5, 9, 9, 0, 0).getTime();
    const yesterday = new Date(2026, 5, 8, 22, 0, 0).getTime();
    const older = new Date(2026, 4, 30, 10, 0, 0).getTime();

    const groups = groupThreadsByDay(
      [thread("a", today), thread("b", yesterday), thread("c", older)],
      now,
    );

    expect(groups.map((group) => group.label)).toEqual(["Today", "Yesterday", "May 30"]);
    expect(groups.map((group) => group.threads.length)).toEqual([1, 1, 1]);
  });

  test("keeps consecutive same-day threads in one group", () => {
    const now = new Date(2026, 5, 9, 15, 0, 0);
    const morning = new Date(2026, 5, 9, 9, 0, 0).getTime();
    const noon = new Date(2026, 5, 9, 12, 0, 0).getTime();

    const groups = groupThreadsByDay([thread("a", noon), thread("b", morning)], now);

    expect(groups).toHaveLength(1);
    expect(groups[0].threads.map((item) => item.id)).toEqual(["a", "b"]);
  });

  test("regroups unsorted input without duplicating day labels", () => {
    const now = new Date(2026, 5, 9, 15, 0, 0);
    const today = new Date(2026, 5, 9, 9, 0, 0).getTime();
    const yesterdayLate = new Date(2026, 5, 8, 22, 0, 0).getTime();
    const yesterdayEarly = new Date(2026, 5, 8, 8, 0, 0).getTime();

    const groups = groupThreadsByDay(
      [thread("b", yesterdayLate), thread("a", today), thread("c", yesterdayEarly)],
      now,
    );

    expect(groups.map((group) => group.label)).toEqual(["Today", "Yesterday"]);
    expect(groups[1].threads.map((item) => item.id)).toEqual(["b", "c"]);
  });
});

describe("neighborThreadId", () => {
  const threads = [thread("a", 3), thread("b", 2), thread("c", 1)];

  test("returns the following thread", () => {
    expect(neighborThreadId(threads, "b")).toBe("c");
  });

  test("returns the previous thread when the last one is removed", () => {
    expect(neighborThreadId(threads, "c")).toBe("b");
  });

  test("returns undefined when it is the only thread", () => {
    expect(neighborThreadId([thread("a", 1)], "a")).toBeUndefined();
  });

  test("returns undefined when the thread is absent", () => {
    expect(neighborThreadId(threads, "missing")).toBeUndefined();
  });
});
