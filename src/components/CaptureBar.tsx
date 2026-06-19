import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { TranscriptionProvider } from "../bindings/TranscriptionProvider";
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
  statusLabel: string;
  fixtureMode: boolean;
  onCancelModelDownload: (provider: TranscriptionProvider) => void;
  onStartModelDownload: (provider: TranscriptionProvider) => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onStartFixtureRecording: () => void;
  onUseWhisper: () => void;
};

export function CaptureBar({
  recorderState,
  meters,
  finalization,
  transcriptionStatus,
  selectedThreadId,
  statusLabel,
  fixtureMode,
  onCancelModelDownload,
  onStartModelDownload,
  onStartRecording,
  onStopRecording,
  onStartFixtureRecording,
  onUseWhisper,
}: CaptureBarProps) {
  const isRecording = recorderState === "recording";
  const busy = recorderState === "starting" || recorderState === "stopping";
  const capturing = isRecording || recorderState === "stopping";
  const finalizationForThread =
    finalization && finalization.threadId === selectedThreadId ? finalization : null;
  const status =
    finalizationForThread?.message ??
    (capturing
      ? "Recording — transcript ready when you stop"
      : (transcriptionStatus?.message ?? "Checking local transcription…"));
  const selectedModel = transcriptionStatus?.availableModels.find((model) => model.selected);
  const missingSelectedModel =
    recorderState === "idle" && Boolean(transcriptionStatus && !transcriptionStatus.ready);
  const activeDownload = Boolean(selectedModel && isModelDownloadActive(selectedModel));
  const whisperInstalled = transcriptionStatus?.availableModels.some(
    (model) => model.provider === "whisper" && model.installed,
  );
  const showWhisperFallback =
    missingSelectedModel && selectedModel?.provider === "parakeet" && Boolean(whisperInstalled);
  const recordButtonLabel = buttonLabel(isRecording, busy, statusLabel, selectedModel);
  const recordButtonAction =
    missingSelectedModel && selectedModel?.canDownload
      ? () => onStartModelDownload(selectedModel.provider)
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
        <button
          type="button"
          className="capture-secondary"
          onClick={() => onCancelModelDownload(selectedModel.provider)}
        >
          Cancel
        </button>
      )}
      {showWhisperFallback && (
        <button type="button" className="capture-secondary" onClick={onUseWhisper}>
          Use Whisper
        </button>
      )}
      {capturing && <time className="capture-elapsed">{formatDuration(meters.elapsedMs)}</time>}
      {capturing && (
        <div className="capture-meters">
          <CaptureMeter label="Mic" level={meters.micLevel} />
          <CaptureMeter label="Sys" level={meters.systemLevel} />
        </div>
      )}
      <span className="capture-status">{status}</span>
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
  return "Record";
}
