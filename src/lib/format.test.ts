import { describe, expect, test } from "bun:test";
import {
  formatBytes,
  formatCompactDuration,
  formatDuration,
  formatMeetingTiming,
} from "./format";

describe("formatDuration", () => {
  test("formats milliseconds as mm:ss", () => {
    expect(formatDuration(0)).toBe("00:00");
    expect(formatDuration(999)).toBe("00:00");
    expect(formatDuration(61_000)).toBe("01:01");
    expect(formatDuration(3_605_000)).toBe("60:05");
  });
});

describe("formatMeetingTiming", () => {
  test("describes upcoming, current, and recently started meetings", () => {
    expect(formatMeetingTiming(300_000, 0)).toBe("starts in 5 min");
    expect(formatMeetingTiming(30_000, 0)).toBe("starts now");
    expect(formatMeetingTiming(0, 61_000)).toBe("started 1 min ago");
  });
});

describe("formatCompactDuration", () => {
  test("uses natural units for header metadata", () => {
    expect(formatCompactDuration(9_000)).toBe("9 sec");
    expect(formatCompactDuration(24 * 60_000 + 18_000)).toBe("24 min");
    expect(formatCompactDuration(65 * 60_000)).toBe("1 hr 5 min");
  });
});

describe("formatBytes", () => {
  test("reports an empty library without a bare zero", () => {
    expect(formatBytes(0)).toBe("0 MB");
    expect(formatBytes(-1)).toBe("0 MB");
  });

  test("agrees with the model catalogue's own size string", () => {
    // PARAKEET_ARCHIVE_BYTES, shown as "460 MB" by the backend.
    expect(formatBytes(482_468_385)).toBe("460 MB");
  });

  test("scales through the units", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(20_480)).toBe("20 KB");
    expect(formatBytes(4_509_715_660)).toBe("4.2 GB");
  });
});
