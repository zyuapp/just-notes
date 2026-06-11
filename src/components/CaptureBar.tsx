import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import { formatDuration } from "../lib/format";
import { CaptureMeter } from "./CaptureMeter";

type CaptureBarProps = {
  recorderState: RecorderState;
  meters: MeterPayload;
  liveStatus: LiveTranscriptStatusPayload | null;
  finalization: FinalizationStatusPayload | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  selectedThreadId: string | null;
  statusLabel: string;
  fixtureMode: boolean;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onStartFixtureRecording: () => void;
};

export function CaptureBar({
  recorderState,
  meters,
  liveStatus,
  finalization,
  transcriptionStatus,
  selectedThreadId,
  statusLabel,
  fixtureMode,
  onStartRecording,
  onStopRecording,
  onStartFixtureRecording,
}: CaptureBarProps) {
  const isRecording = recorderState === "recording";
  const busy = recorderState === "starting" || recorderState === "stopping";
  const live = isRecording || recorderState === "stopping";
  const finalizationForThread =
    finalization && finalization.threadId === selectedThreadId ? finalization : null;
  const status =
    finalizationForThread?.message ??
    (live
      ? (liveStatus?.message ?? "Transcribing locally…")
      : (transcriptionStatus?.message ?? "Checking local transcription…"));

  return (
    <footer className={isRecording ? "capture-bar recording" : "capture-bar"}>
      <button
        type="button"
        className="record-button"
        onClick={isRecording ? onStopRecording : onStartRecording}
        disabled={busy}
        aria-label={statusLabel}
      >
        <span className="record-glyph" aria-hidden="true" />
        <span>{isRecording ? "Stop" : busy ? `${statusLabel}…` : "Record"}</span>
      </button>
      {live && <time className="capture-elapsed">{formatDuration(meters.elapsedMs)}</time>}
      {live && (
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
