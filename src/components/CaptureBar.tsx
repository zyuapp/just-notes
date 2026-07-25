import type { MeterPayload } from "../bindings/MeterPayload";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import type { RecordButtonControl } from "../features/app/recordButtonState";
import { formatDuration } from "../lib/format";
import { CaptureMeter } from "./CaptureMeter";
import { Button } from "./Button";
import { DownloadingLabel } from "./DownloadingLabel";

type CaptureBarProps = {
  recorderState: RecorderState;
  meters: MeterPayload;
  transcriptionStatus: TranscriptionStatusPayload | null;
  recordButtonControl: RecordButtonControl;
  fixtureMode: boolean;
  onStartFixtureRecording: () => void;
};

export function CaptureBar({
  recorderState,
  meters,
  transcriptionStatus,
  recordButtonControl,
  fixtureMode,
  onStartFixtureRecording,
}: CaptureBarProps) {
  const isRecording = recorderState === "recording";
  const capturing = isRecording || recorderState === "stopping";
  const idleStatus =
    transcriptionStatus == null
      ? "Checking local transcription…"
      : transcriptionStatus.ready
        ? null
        : transcriptionStatus.message;
  const status = capturing ? "Recording — transcript ready when you stop" : idleStatus;

  return (
    <footer className={isRecording ? "capture-bar recording" : "capture-bar"}>
      <button
        type="button"
        className="record-button"
        onClick={recordButtonControl.onClick}
        disabled={recordButtonControl.disabled}
        aria-label={recordButtonControl.ariaLabel}
      >
        <span className="record-glyph" aria-hidden="true" />
        <span>
          {recordButtonControl.progressPercent != null ? (
            <DownloadingLabel percent={recordButtonControl.progressPercent} />
          ) : (
            recordButtonControl.label
          )}
        </span>
      </button>
      {recordButtonControl.onCancelDownload && (
        <Button onClick={recordButtonControl.onCancelDownload}>
          Cancel
        </Button>
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
