import { describe, expect, test } from "bun:test";
import {
  clampSidebarWidth,
  DEFAULT_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
  MIN_SIDEBAR_WIDTH,
} from "./sidebarWidth";

describe("clampSidebarWidth", () => {
  test("keeps widths within the resize bounds", () => {
    expect(clampSidebarWidth(MIN_SIDEBAR_WIDTH - 50)).toBe(MIN_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(MAX_SIDEBAR_WIDTH + 50)).toBe(MAX_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(300)).toBe(300);
  });

  test("rounds fractional drag positions to whole pixels", () => {
    expect(clampSidebarWidth(280.6)).toBe(281);
  });

  test("falls back to the default for non-finite input", () => {
    expect(clampSidebarWidth(Number.NaN)).toBe(DEFAULT_SIDEBAR_WIDTH);
  });

  test("default width sits inside the bounds", () => {
    expect(DEFAULT_SIDEBAR_WIDTH).toBeGreaterThanOrEqual(MIN_SIDEBAR_WIDTH);
    expect(DEFAULT_SIDEBAR_WIDTH).toBeLessThanOrEqual(MAX_SIDEBAR_WIDTH);
  });
});
