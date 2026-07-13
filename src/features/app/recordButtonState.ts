import type { TranscriptionModelStatus } from "../../bindings/TranscriptionModelStatus";
import {
  downloadActionLabel,
  downloadPercent,
  isModelDownloadActive,
} from "../../lib/transcriptionModel";
import { getStatusLabel, type AppState, type RecorderState } from "./state";

export type RecordButtonAction = "start" | "stop" | "download";

export type RecordButtonState = {
  label: string;
  progressPercent: number | null;
  ariaLabel: string;
  action: RecordButtonAction | null;
  disabled: boolean;
};

export type RecordButtonControl = RecordButtonState & {
  onClick: (() => void) | undefined;
  onCancelDownload: (() => void) | undefined;
};

export function buildRecordButtonControl(
  state: AppState,
  commands: {
    startRecording: () => void;
    stopRecording: () => void;
    startModelDownload: () => void;
    cancelModelDownload: () => void;
  },
): RecordButtonControl {
  const selectedModel = state.transcriptionStatus?.availableModels.find((model) => model.selected);
  const buttonState = buildRecordButtonState({
    recorderState: state.recorderState,
    statusLabel: getStatusLabel(state),
    selectedModel,
    transcriptionReady: state.transcriptionStatus?.ready,
    resumeSelected:
      state.recorderState === "idle" &&
      state.selectedThread?.summary.status === "idle" &&
      state.selectedThread.segments.length > 0,
  });
  const onClick =
    buttonState.action === "stop"
      ? commands.stopRecording
      : buttonState.action === "download"
        ? commands.startModelDownload
        : buttonState.action === "start"
          ? commands.startRecording
          : undefined;
  const onCancelDownload =
    selectedModel && isModelDownloadActive(selectedModel) && selectedModel.canCancel
      ? commands.cancelModelDownload
      : undefined;
  return { ...buttonState, onClick, onCancelDownload };
}

export function buildRecordButtonState(params: {
  recorderState: RecorderState;
  statusLabel: string;
  selectedModel: TranscriptionModelStatus | undefined;
  transcriptionReady: boolean | undefined;
  resumeSelected: boolean;
}): RecordButtonState {
  if (params.recorderState === "recording") {
    return control("Stop", "Stop recording", "stop");
  }
  if (params.recorderState === "starting" || params.recorderState === "stopping") {
    return control(`${params.statusLabel}…`, "Recording status changing", null, true);
  }
  if (params.transcriptionReady === false && !params.selectedModel) {
    return control("Record", "Transcription model unavailable", null, true);
  }
  if (!params.selectedModel?.installed && params.selectedModel?.downloadable) {
    return modelDownloadState(params.selectedModel, params.transcriptionReady);
  }
  return control(
    params.resumeSelected ? "Resume" : "Record",
    params.resumeSelected ? "Resume recording" : "Start recording",
    "start",
  );
}

function modelDownloadState(
  model: TranscriptionModelStatus,
  transcriptionReady: boolean | undefined,
): RecordButtonState {
  if (model.downloadState === "downloading") {
    return {
      ...control("Downloading", "Downloading transcription model", null, true),
      progressPercent: downloadPercent(model),
    };
  }
  if (model.downloadState === "installing") {
    return control("Installing", "Installing transcription model", null, true);
  }
  const modelRequired = transcriptionReady === false;
  return control(
    downloadActionLabel(model),
    "Download transcription model",
    modelRequired ? (model.canDownload ? "download" : null) : "start",
    modelRequired && !model.canDownload,
  );
}

function control(
  label: string,
  ariaLabel: string,
  action: RecordButtonAction | null,
  disabled = false,
): RecordButtonState {
  return { label, progressPercent: null, ariaLabel, action, disabled };
}
