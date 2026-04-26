import { Circle, FlaskConical, Square } from "lucide-react";
import type { RecorderState } from "../features/app/state";

type RecorderControlsProps = {
  fixtureMode: boolean;
  recorderState: RecorderState;
  statusLabel: string;
  onStart: () => void;
  onStartFixture: () => void;
  onStop: () => void;
};

export function RecorderControls({
  fixtureMode,
  recorderState,
  statusLabel,
  onStart,
  onStartFixture,
  onStop,
}: RecorderControlsProps) {
  const isRecording = recorderState === "recording";
  const canStart = recorderState === "idle";

  return (
    <div className="sidebar-actions">
      <button
        type="button"
        className={isRecording ? "record-button recording" : "record-button"}
        onClick={isRecording ? onStop : onStart}
        disabled={recorderState === "starting" || recorderState === "stopping"}
        aria-label={isRecording ? "Stop recording" : "Start recording"}
      >
        {isRecording ? (
          <Square size={16} aria-hidden="true" />
        ) : (
          <Circle size={16} aria-hidden="true" />
        )}
        <span>{isRecording ? "Stop" : "Record"}</span>
      </button>
      <div className="capture-state">{statusLabel}</div>
      {fixtureMode && (
        <button
          type="button"
          className="fixture-button"
          onClick={onStartFixture}
          disabled={!canStart}
          aria-label="Start QA fixture recording"
        >
          <FlaskConical size={15} aria-hidden="true" />
          <span>QA fixture</span>
        </button>
      )}
    </div>
  );
}
