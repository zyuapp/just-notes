import { describe, expect, test } from "bun:test";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import {
  downloadActionLabel,
  downloadConfirmationMessage,
  downloadPercent,
  isModelDownloadActive,
} from "./transcriptionModel";

function model(overrides: Partial<TranscriptionModelStatus>): TranscriptionModelStatus {
  return {
    name: "Parakeet TDT 0.6B v2",
    filename: "parakeet",
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
  test("labels the Parakeet download", () => {
    expect(downloadActionLabel()).toBe("Download Parakeet");
    expect(downloadActionLabel(model({ displaySize: "460 MB" }))).toBe(
      "Download Parakeet · 460 MB",
    );
  });

  test("builds a size-aware download consent message", () => {
    expect(downloadConfirmationMessage(model({ displaySize: "460 MB" }))).toBe(
      "Local transcription requires a one-time 460 MB download. Audio stays on this Mac.",
    );
  });
});

describe("isModelDownloadActive", () => {
  test("true while downloading or installing", () => {
    expect(isModelDownloadActive(model({ downloadState: "downloading" }))).toBe(true);
    expect(isModelDownloadActive(model({ downloadState: "installing" }))).toBe(true);
  });

  test("false when idle, failed, or cancelled", () => {
    for (const downloadState of ["idle", "failed", "cancelled"] as const) {
      expect(isModelDownloadActive(model({ downloadState }))).toBe(false);
    }
  });
});
