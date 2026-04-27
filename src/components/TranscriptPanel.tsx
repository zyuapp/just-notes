import { FileText } from "lucide-react";
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
  liveStatus: LiveTranscriptStatusPayload | null;
  meters: MeterPayload;
  onCreateThread: () => void;
  onStartRecording: () => void;
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
};

export function TranscriptPanel({
  error,
  liveStatus,
  meters,
  onCreateThread,
  onStartRecording,
  recorderState,
  selectedThread,
  transcriptionStatus,
}: TranscriptPanelProps) {
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

      <div className="engine-row">
        <FileText size={15} aria-hidden="true" />
        <span>{transcriptionStatus?.message ?? "Checking local transcription"}</span>
      </div>

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
