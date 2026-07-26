import { describe, expect, test } from "bun:test";
import type { TranscriptionModelStatus } from "../../bindings/TranscriptionModelStatus";
import { buildRecordButtonControl } from "./recordButtonState";
import { initialAppState, type AppState } from "./state";

const startRecording = () => {};
const stopRecording = () => {};
const startModelDownload = () => {};
const cancelModelDownload = () => {};
const commands = { startRecording, stopRecording, startModelDownload, cancelModelDownload };

function model(overrides: Partial<TranscriptionModelStatus> = {}): TranscriptionModelStatus {
  return {
    name: "Parakeet",
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

function state(overrides: Partial<AppState>): AppState {
  return { ...initialAppState, ...overrides };
}

function transcriptionStatus(selectedModel: TranscriptionModelStatus, ready = false) {
  return {
    ready,
    engineExists: true,
    modelExists: ready,
    enginePath: "/engine",
    modelPath: selectedModel.path,
    modelName: selectedModel.name,
    availableModels: [selectedModel],
    message: ready ? "Ready" : "Model required",
  };
}

describe("buildRecordButtonControl", () => {
  test("maps recorder and model states to the owning commands", () => {
    expect(buildRecordButtonControl(state({ recorderState: "recording" }), commands).onClick).toBe(
      stopRecording,
    );
    expect(
      buildRecordButtonControl(
        state({ transcriptionStatus: transcriptionStatus(model()) }),
        commands,
      ).onClick,
    ).toBe(startModelDownload);
    expect(
      buildRecordButtonControl(
        state({ transcriptionStatus: transcriptionStatus(model({ installed: true }), true) }),
        commands,
      ).onClick,
    ).toBe(startRecording);
  });

  test("derives resume and cancel controls from app state", () => {
    const resume = buildRecordButtonControl(
      state({
        selectedThread: {
          summary: {
            id: "thread", title: "Thread", createdAtMs: 0, updatedAtMs: 0,
            status: "idle", segmentCount: 1, durationMs: 100, snippet: "Hello",
            hasAudio: true, path: "/thread", calendar: null,
          },
          segments: [{ speaker: "You", source: "mic", startMs: 0, endMs: 100, text: "Hello" }],
          transcriptMarkdownPath: "/thread/transcript.md",
        },
        transcriptionStatus: transcriptionStatus(model({ installed: true }), true),
      }),
      commands,
    );
    expect(resume).toMatchObject({ label: "Resume", onClick: startRecording });

    const cancellableModel = model({ downloadState: "downloading", canCancel: true });
    const cancellable = buildRecordButtonControl(
      state({ transcriptionStatus: transcriptionStatus(cancellableModel) }), commands,
    );
    expect(cancellable.onCancelDownload).toBe(cancelModelDownload);
    const notCancellable = buildRecordButtonControl(
      state({ transcriptionStatus: transcriptionStatus({ ...cancellableModel, canCancel: false }) }),
      commands,
    );
    expect(notCancellable.onCancelDownload).toBeUndefined();
  });
});
