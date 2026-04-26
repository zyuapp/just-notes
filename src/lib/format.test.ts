import { describe, expect, test } from "bun:test";
import { formatDuration } from "./format";

describe("formatDuration", () => {
  test("formats milliseconds as mm:ss", () => {
    expect(formatDuration(0)).toBe("00:00");
    expect(formatDuration(999)).toBe("00:00");
    expect(formatDuration(61_000)).toBe("01:01");
    expect(formatDuration(3_605_000)).toBe("60:05");
  });
});
