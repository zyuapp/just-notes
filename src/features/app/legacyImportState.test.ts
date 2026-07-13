import { expect, test } from "bun:test";
import { openLegacyImportState, type LegacyImportViewState } from "./legacyImportState";

test("opens a new legacy import from the intro", () => {
  expect(openLegacyImportState(null)).toEqual({ stage: "intro" });
});

test("ignores menu re-entry while a legacy import is active", () => {
  const active: LegacyImportViewState = { stage: "locating" };
  expect(openLegacyImportState(active)).toBe(active);
});
