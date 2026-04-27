import { Circle, FileText, Mic, Radio, Square } from "lucide-react";
import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import { formatDuration, formatThreadDate } from "../lib/format";
import { ErrorToast } from "./ErrorToast";
import { PanelFooter } from "./PanelFooter";
import { TranscriptSurface } from "./TranscriptSurface";

type TranscriptPanelProps = {
  error: string | null;
  fixtureMode: boolean;
  liveStatus: LiveTranscriptStatusPayload | null;
  meters: MeterPayload;
  onCreateThread: () => void;
  onStartFixtureRecording: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  statusLabel: string;
  transcriptionStatus: TranscriptionStatusPayload | null;
};

export function TranscriptPanel({
  error,
  fixtureMode,
  liveStatus,
  meters,
  onCreateThread,
  onStartFixtureRecording,
  onStartRecording,
  onStopRecording,
  recorderState,
  selectedThread,
  statusLabel,
  transcriptionStatus,
}: TranscriptPanelProps) {
  const isRecording = recorderState === "recording";

  return (
    <section className="thread-panel" aria-label="Transcript">
      <header className="panel-head">
        <div>
          <p className="eyebrow">
            {selectedThread ? formatThreadDate(selectedThread.summary.createdAtMs) : "Local"}
          </p>
          <h1>{selectedThread?.summary.title ?? "No thread selected"}</h1>
        </div>
        <div className="panel-status">
          <span className={recorderState === "recording" ? "status-light live" : "status-light"} />
          <span>{formatDuration(meters.elapsedMs)}</span>
        </div>
      </header>

      <section className="capture-strip" aria-label="Capture controls">
        <button
          type="button"
          className={isRecording ? "capture-record recording" : "capture-record"}
          onClick={isRecording ? onStopRecording : onStartRecording}
          disabled={recorderState === "starting" || recorderState === "stopping"}
          aria-label={statusLabel}
        >
          {isRecording ? <Square size={24} aria-hidden="true" /> : <Circle size={24} aria-hidden="true" />}
          <span>{isRecording ? "Stop" : "Record"}</span>
        </button>
        <CaptureMeter icon="mic" label="Mic" level={meters.micLevel} />
        <CaptureMeter icon="system" label="System" level={meters.systemLevel} />
        <div className="engine-row">
          <FileText size={21} aria-hidden="true" />
          <span>{transcriptionStatus?.message ?? "Local transcription is ready (small.en)"}</span>
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
      </section>

      <TranscriptSurface
        recorderState={recorderState}
        selectedThread={selectedThread}
        onCreateThread={onCreateThread}
        onStartRecording={onStartRecording}
      />
      <PanelFooter liveStatus={liveStatus} selectedThread={selectedThread} />
      <ErrorToast message={error} />
    </section>
  );
}

type CaptureMeterProps = {
  icon: "mic" | "system";
  label: string;
  level: number;
};

const captureBars = Array.from({ length: 24 }, (_, index) => index);

function CaptureMeter({ icon, label, level }: CaptureMeterProps) {
  const Icon = icon === "mic" ? Mic : Radio;
  const activeBars = Math.round(Math.min(1, Math.max(0, level)) * captureBars.length);

  return (
    <section className="capture-meter" aria-label={`${label} level`}>
      <div className="capture-meter-head">
        <Icon size={20} aria-hidden="true" />
        <span>{label}</span>
        <strong>{Math.round(level * 100)}%</strong>
      </div>
      <div className="capture-meter-bars" aria-hidden="true">
        {captureBars.map((bar) => (
          <span key={bar} className={bar < activeBars ? "active" : ""} />
        ))}
      </div>
    </section>
  );
}
