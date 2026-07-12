import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import { formatDuration } from "../lib/format";
import {
  downloadActionLabel,
  downloadPercent,
  isModelDownloadActive,
} from "../lib/transcriptionModel";
import { CaptureMeter } from "./CaptureMeter";
import { DownloadingLabel } from "./DownloadingLabel";

type CaptureBarProps = {
  recorderState: RecorderState;
  meters: MeterPayload;
  finalization: FinalizationStatusPayload | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  selectedThreadId: string | null;
  resumeSelected: boolean;
  statusLabel: string;
  fixtureMode: boolean;
  onCancelModelDownload: () => void;
  onStartModelDownload: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onStartFixtureRecording: () => void;
};

export function CaptureBar({
  recorderState,
  meters,
  finalization,
  transcriptionStatus,
  selectedThreadId,
  resumeSelected,
  statusLabel,
  fixtureMode,
  onCancelModelDownload,
  onStartModelDownload,
  onStartRecording,
  onStopRecording,
  onStartFixtureRecording,
}: CaptureBarProps) {
  const isRecording = recorderState === "recording";
  const busy = recorderState === "starting" || recorderState === "stopping";
  const capturing = isRecording || recorderState === "stopping";
  const finalizationForThread =
    finalization && finalization.threadId === selectedThreadId ? finalization : null;
  const idleStatus =
    transcriptionStatus == null
      ? "Checking local transcription…"
      : transcriptionStatus.ready
        ? null
        : transcriptionStatus.message;
  const status =
    finalizationForThread?.message ??
    (capturing ? "Recording — transcript ready when you stop" : idleStatus);
  const selectedModel = transcriptionStatus?.availableModels.find((model) => model.selected);
  const missingSelectedModel =
    recorderState === "idle" && Boolean(transcriptionStatus && !transcriptionStatus.ready);
  const activeDownload = Boolean(selectedModel && isModelDownloadActive(selectedModel));
  const recordButtonLabel = buttonLabel(
    isRecording,
    busy,
    statusLabel,
    selectedModel,
    resumeSelected,
  );
  const recordButtonAction =
    missingSelectedModel && selectedModel?.canDownload
      ? onStartModelDownload
      : isRecording
        ? onStopRecording
        : onStartRecording;

  return (
    <footer className={isRecording ? "capture-bar recording" : "capture-bar"}>
      <button
        type="button"
        className="record-button"
        onClick={recordButtonAction}
        disabled={busy || (missingSelectedModel && !selectedModel?.canDownload)}
        aria-label={statusLabel}
      >
        <span className="record-glyph" aria-hidden="true" />
        <span>{recordButtonLabel}</span>
      </button>
      {activeDownload && selectedModel?.canCancel && (
        <button type="button" className="capture-secondary" onClick={onCancelModelDownload}>
          Cancel
        </button>
      )}
      {capturing && <time className="capture-elapsed">{formatDuration(meters.elapsedMs)}</time>}
      {capturing && (
        <div className="capture-meters">
          <CaptureMeter label="Mic" level={meters.micLevel} />
          <CaptureMeter label="Sys" level={meters.systemLevel} />
        </div>
      )}
      {status && <span className="capture-status">{status}</span>}
      {fixtureMode && (
        <button
          type="button"
          className="fixture-link"
          onClick={onStartFixtureRecording}
          disabled={recorderState !== "idle"}
        >
          QA fixture
        </button>
      )}
    </footer>
  );
}

function buttonLabel(
  isRecording: boolean,
  busy: boolean,
  statusLabel: string,
  selectedModel: TranscriptionModelStatus | undefined,
  resumeSelected: boolean,
) {
  if (isRecording) return "Stop";
  if (busy) return `${statusLabel}…`;
  if (!selectedModel?.installed && selectedModel?.downloadable) {
    if (selectedModel.downloadState === "downloading") {
      return <DownloadingLabel percent={downloadPercent(selectedModel)} />;
    }
    if (selectedModel.downloadState === "installing") return "Installing";
    return downloadActionLabel(selectedModel);
  }
  return resumeSelected ? "Resume" : "Record";
}
