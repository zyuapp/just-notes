import { Mic, Volume2 } from "lucide-react";
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
  const statusLabel = recorderState === "stopping" ? "Finishing recording"
    : isRecording ? "Recording" : recorderState === "starting" ? "Starting recording"
      : transcriptionStatus?.ready ? "Ready" : "Transcription";

  return (
    <footer className="capture-footer">
      <div className={isRecording ? "capture-bar recording" : "capture-bar"}>
        <div className="capture-state">
          <span className={transcriptionStatus?.ready ? "capture-indicator ready" : "capture-indicator"} aria-hidden="true" />
          <div className="capture-copy">
            <strong>{statusLabel}</strong>
            {!capturing && idleStatus && <span className="capture-status">{idleStatus}</span>}
          </div>
          {capturing && <time className="capture-elapsed">{formatDuration(meters.elapsedMs)}</time>}
        </div>
        {capturing ? (
          <div className="capture-meters">
            <CaptureMeter label="Mic" level={meters.micLevel} />
            <CaptureMeter label="Sys" level={meters.systemLevel} />
          </div>
        ) : (
          <div className="capture-sources">
            <span><Mic size={13} aria-hidden="true" />Microphone</span>
            <span><Volume2 size={13} aria-hidden="true" />System audio</span>
          </div>
        )}
        <div className="capture-actions">
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
            <Button onClick={recordButtonControl.onCancelDownload}>Cancel</Button>
          )}
        </div>
      </div>
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
