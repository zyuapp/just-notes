import { expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { CaptureBar } from "./CaptureBar";
import { TranscriptActions } from "./TranscriptActions";
import { buildRecordButtonControl } from "../features/app/recordButtonState";
import { initialAppState, type AppState } from "../features/app/state";

test("recording controls retain command routing, download cancellation, and busy guards", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previous = { window: globalThis.window, document: globalThis.document,
    navigator: globalThis.navigator, IS_REACT_ACT_ENVIRONMENT: globalThis.IS_REACT_ACT_ENVIRONMENT };
  Object.assign(globalThis, { window: browserWindow, document: browserWindow.document,
    navigator: browserWindow.navigator, IS_REACT_ACT_ENVIRONMENT: true });
  const container = document.createElement("div");
  const root = createRoot(container);
  const commands = { startRecording: mock(), stopRecording: mock(),
    startModelDownload: mock(), cancelModelDownload: mock() };
  const state: AppState = { ...initialAppState, meters: {
    threadId: "active", elapsedMs: 65_000, micLevel: .5, systemLevel: 1,
  } };
  const render = async () => act(async () => root.render(createElement(CaptureBar, {
    recorderState: state.recorderState, meters: state.meters,
    transcriptionStatus: state.transcriptionStatus,
    recordButtonControl: buildRecordButtonControl(state, commands),
    fixtureMode: false, onStartFixtureRecording: mock(),
  })));
  const record = () => container.querySelector<HTMLButtonElement>(".record-button")!;
  try {
    await render();
    record().click();
    expect(commands.startRecording).toHaveBeenCalledTimes(1);

    state.recorderState = "recording";
    await render();
    expect(container.querySelector(".capture-elapsed")?.textContent).toBe("01:05");
    expect(container.querySelectorAll(".capture-meter")).toHaveLength(2);
    expect(container.querySelectorAll(".capture-meter:first-child .on")).toHaveLength(7);
    record().click();
    expect(commands.stopRecording).toHaveBeenCalledTimes(1);

    for (const busyState of ["starting", "stopping"] as const) {
      state.recorderState = busyState;
      await render();
      expect(record().disabled).toBe(true);
      record().click();
    }
    expect(commands.startRecording).toHaveBeenCalledTimes(1);
    expect(commands.stopRecording).toHaveBeenCalledTimes(1);

    state.recorderState = "idle";
    const model = { name: "Parakeet", filename: "model", path: "/model", installed: false,
      selected: true, downloadable: true, downloadState: "idle" as const,
      progressBytes: 0, totalBytes: 100, displaySize: "100 MB", canDownload: true,
      canCancel: false, errorMessage: null };
    state.transcriptionStatus = { ready: false, engineExists: true, modelExists: false,
      enginePath: "/engine", modelPath: "/model", modelName: "Parakeet",
      availableModels: [model], message: "Download the transcription model." };
    await render();
    record().click();
    expect(commands.startModelDownload).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain(state.transcriptionStatus.message);

    state.transcriptionStatus.availableModels = [{ ...model, downloadState: "downloading",
      progressBytes: 42, canDownload: false, canCancel: true }];
    await render();
    expect(record().disabled).toBe(true);
    expect(record().textContent).toContain("42%");
    [...container.querySelectorAll("button")].find((button) => button.textContent === "Cancel")?.click();
    expect(commands.cancelModelDownload).toHaveBeenCalledTimes(1);

    const copy = mock(), archive = mock();
    const toolbar = async (canModify: boolean) => act(async () => root.render(
      createElement(TranscriptActions, { canModify,
        onCopy: copy, onArchive: archive })));
    await toolbar(false);
    container.querySelector<HTMLButtonElement>('[aria-label="Copy transcript"]')!.click();
    const archiveButton = container.querySelector<HTMLButtonElement>('[aria-label="Archive thread"]')!;
    expect(archiveButton.disabled).toBe(true);
    archiveButton.click();
    expect(copy).toHaveBeenCalledTimes(1);
    expect(archive).toHaveBeenCalledTimes(0);
    await toolbar(true);
    container.querySelector<HTMLButtonElement>('[aria-label="Archive thread"]')!.click();
    expect(archive).toHaveBeenCalledTimes(1);
  } finally {
    await act(async () => root.unmount());
    browserWindow.close();
    Object.assign(globalThis, previous);
  }
});
