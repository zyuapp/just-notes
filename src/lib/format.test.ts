import { describe, expect, test } from "bun:test";
import { formatCompactDuration, formatDuration } from "./format";

describe("formatDuration", () => {
  test("formats milliseconds as mm:ss", () => {
    expect(formatDuration(0)).toBe("00:00");
    expect(formatDuration(999)).toBe("00:00");
    expect(formatDuration(61_000)).toBe("01:01");
    expect(formatDuration(3_605_000)).toBe("60:05");
  });
});

describe("formatCompactDuration", () => {
  test("uses natural units for header metadata", () => {
    expect(formatCompactDuration(9_000)).toBe("9 sec");
    expect(formatCompactDuration(24 * 60_000 + 18_000)).toBe("24 min");
    expect(formatCompactDuration(65 * 60_000)).toBe("1 hr 5 min");
  });
});
