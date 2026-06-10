import { describe, expect, test } from "bun:test";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { groupThreadsByDay } from "./threads";

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
});
