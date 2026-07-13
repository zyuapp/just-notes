import { describe, expect, test } from "bun:test";
import type { TranscriptionModelStatus } from "../../bindings/TranscriptionModelStatus";
import type { RecorderState } from "./state";
import { buildRecordButtonState } from "./recordButtonState";

function model(overrides: Partial<TranscriptionModelStatus> = {}): TranscriptionModelStatus {
  return {
    name: "Parakeet TDT 0.6B v2",
    filename: "parakeet",
    path: "/models/parakeet",
    installed: false,
    selected: true,
    downloadable: true,
    downloadState: "idle",
    progressBytes: 0,
    totalBytes: 100,
    displaySize: "460 MB",
    canDownload: true,
    canCancel: false,
    errorMessage: null,
    ...overrides,
  };
}

function presentation(overrides: {
  recorderState?: RecorderState;
  selectedModel?: TranscriptionModelStatus | null;
  transcriptionReady?: boolean;
  resumeSelected?: boolean;
} = {}) {
  return buildRecordButtonState({
    recorderState: overrides.recorderState ?? "idle",
    statusLabel: overrides.recorderState === "stopping" ? "Stopping" : "Starting",
    selectedModel:
      overrides.selectedModel === null
        ? undefined
        : (overrides.selectedModel ?? model({ installed: true })),
    transcriptionReady: overrides.transcriptionReady ?? true,
    resumeSelected: overrides.resumeSelected ?? false,
  });
}

describe("buildRecordButtonState", () => {
  test("maps recorder transitions to the correct control behavior", () => {
    expect(presentation({ recorderState: "recording" })).toMatchObject({
      label: "Stop",
      ariaLabel: "Stop recording",
      action: "stop",
      disabled: false,
    });
    expect(presentation({ recorderState: "starting" })).toMatchObject({
      label: "Starting…",
      action: null,
      disabled: true,
    });
    expect(presentation({ recorderState: "stopping" })).toMatchObject({
      label: "Stopping…",
      action: null,
      disabled: true,
    });
  });

  test("starts or resumes when the installed model is ready", () => {
    expect(presentation()).toMatchObject({
      label: "Record",
      ariaLabel: "Start recording",
      action: "start",
      disabled: false,
    });
    expect(presentation({ resumeSelected: true })).toMatchObject({
      label: "Resume",
      ariaLabel: "Resume recording",
      action: "start",
      disabled: false,
    });
  });

  test("downloads a missing model and disables an unavailable one", () => {
    expect(presentation({ selectedModel: model(), transcriptionReady: false })).toMatchObject({
      label: "Download Parakeet · 460 MB",
      action: "download",
      disabled: false,
    });
    expect(
      presentation({
        selectedModel: model({ canDownload: false }),
        transcriptionReady: false,
      }),
    ).toMatchObject({ action: null, disabled: true });
    expect(presentation({ selectedModel: null, transcriptionReady: false })).toMatchObject({
      label: "Record",
      ariaLabel: "Transcription model unavailable",
      action: null,
      disabled: true,
    });
  });

  test("shows download progress and installation without enabling the button", () => {
    expect(
      presentation({
        selectedModel: model({ downloadState: "downloading", progressBytes: 42 }),
        transcriptionReady: false,
      }),
    ).toMatchObject({
      label: "Downloading",
      progressPercent: 42,
      ariaLabel: "Downloading transcription model",
      disabled: true,
    });
    expect(
      presentation({
        selectedModel: model({ downloadState: "installing" }),
        transcriptionReady: false,
      }),
    ).toMatchObject({
      label: "Installing",
      ariaLabel: "Installing transcription model",
      disabled: true,
    });
  });

  test("allows failed and cancelled downloads to be retried", () => {
    for (const downloadState of ["failed", "cancelled"] as const) {
      expect(
        presentation({ selectedModel: model({ downloadState }), transcriptionReady: false }),
      ).toMatchObject({ action: "download", disabled: false });
    }
  });
});
