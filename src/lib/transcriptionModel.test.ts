import { describe, expect, test } from "bun:test";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import { downloadActionLabel, downloadPercent } from "./transcriptionModel";

function model(overrides: Partial<TranscriptionModelStatus>): TranscriptionModelStatus {
  return {
    name: "Parakeet TDT 0.6B v2",
    filename: "parakeet",
    provider: "parakeet",
    path: "/models/parakeet",
    installed: false,
    selected: true,
    downloadable: true,
    downloadState: "idle",
    progressBytes: 0,
    totalBytes: 0,
    displaySize: "460 MB",
    canDownload: true,
    canCancel: false,
    errorMessage: null,
    ...overrides,
  };
}

describe("downloadPercent", () => {
  test("returns 0 when the total is unknown", () => {
    expect(downloadPercent(model({ progressBytes: 10, totalBytes: 0 }))).toBe(0);
  });

  test("floors the percentage", () => {
    expect(downloadPercent(model({ progressBytes: 1, totalBytes: 3 }))).toBe(33);
  });

  test("clamps to 100", () => {
    expect(downloadPercent(model({ progressBytes: 200, totalBytes: 100 }))).toBe(100);
  });
});

describe("downloadActionLabel", () => {
  test("uses the short label for Parakeet", () => {
    expect(downloadActionLabel(model({ provider: "parakeet" }))).toBe("Download Parakeet");
  });

  test("uses the model name for other providers", () => {
    expect(downloadActionLabel(model({ provider: "whisper", name: "small.en" }))).toBe(
      "Download small.en",
    );
  });
});
