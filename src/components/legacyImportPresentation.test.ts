import { describe, expect, test } from "bun:test";
import {
  completeImportSummary,
  formatImportBytes,
  importActionLabel,
} from "./legacyImportPresentation";

describe("importActionLabel", () => {
  test("handles progress and zero, one, or many recordings", () => {
    expect(importActionLabel(true, 3)).toBe("Importing…");
    expect(importActionLabel(false, 0)).toBe("Finish import");
    expect(importActionLabel(false, 1)).toBe("Import 1 recording");
    expect(importActionLabel(false, 2)).toBe("Import 2 recordings");
  });
});

describe("completeImportSummary", () => {
  test("includes only nonzero duplicate and conflict clauses", () => {
    expect(
      completeImportSummary({
        imported: 1,
        activeRecordings: 1,
        archivedRecordings: 0,
        duplicates: 0,
        conflicts: 0,
      }),
    ).toBe("1 recording imported");
    expect(
      completeImportSummary({
        imported: 3,
        activeRecordings: 2,
        archivedRecordings: 1,
        duplicates: 1,
        conflicts: 2,
      }),
    ).toBe("3 recordings imported · 1 duplicate skipped · 2 conflicts preserved");
  });
});

describe("formatImportBytes", () => {
  test("formats byte-unit boundaries", () => {
    expect(formatImportBytes(0)).toBe("1 KB");
    expect(formatImportBytes(1024 * 1024 - 1)).toBe("1024 KB");
    expect(formatImportBytes(1024 * 1024)).toBe("1 MB");
    expect(formatImportBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
  });
});
