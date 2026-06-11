import { Circle, FileText, Square } from "lucide-react";
import { useState } from "react";
import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import type { ThreadActions } from "../features/app/useThreadActions";
import { formatDuration, formatThreadDate } from "../lib/format";
import { CaptureMeter } from "./CaptureMeter";
import { ErrorToast } from "./ErrorToast";
import { NoticeBar, type Notice } from "./NoticeBar";
import { PanelFooter } from "./PanelFooter";
import { ThreadTitle } from "./ThreadTitle";
import { TranscriptSurface } from "./TranscriptSurface";
import { TranscriptToolbar } from "./TranscriptToolbar";

type TranscriptPanelProps = {
  error: string | null;
  fixtureMode: boolean;
  liveStatus: LiveTranscriptStatusPayload | null;
  finalization: FinalizationStatusPayload | null;
  meters: MeterPayload;
  notice: Notice | null;
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  statusLabel: string;
  transcriptionStatus: TranscriptionStatusPayload | null;
  threadActions: ThreadActions;
  onCreateThread: () => void;
  onStartFixtureRecording: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
};

export function TranscriptPanel({
  error,
  fixtureMode,
  liveStatus,
  finalization,
  meters,
  notice,
  recorderState,
  selectedThread,
  statusLabel,
  transcriptionStatus,
  threadActions,
  onCreateThread,
  onStartFixtureRecording,
  onStartRecording,
  onStopRecording,
}: TranscriptPanelProps) {
  const [query, setQuery] = useState("");
  const isRecording = recorderState === "recording";
  const liveElapsed = isRecording || recorderState === "stopping";
  const elapsedMs = liveElapsed ? meters.elapsedMs : (selectedThread?.summary.durationMs ?? 0);
  const hasSegments = (selectedThread?.segments.length ?? 0) > 0;
  const canModify = recorderState === "idle" && selectedThread?.summary.status === "idle";
  const speakers = Array.from(
    new Set(selectedThread?.segments.map((segment) => segment.speaker) ?? []),
  );

  return (
    <section className="thread-panel" aria-label="Transcript">
      <header className="panel-head">
        <div>
          <p className="eyebrow">
            {selectedThread
              ? `${formatThreadDate(selectedThread.summary.createdAtMs)}${hasSegments || selectedThread.summary.hasAudio ? " · Mic + System" : ""}`
              : "Local"}
          </p>
          <ThreadTitle
            title={selectedThread?.summary.title ?? null}
            canRename={canModify}
            onRename={(title) => void threadActions.renameThread(title)}
          />
        </div>
        <div className="panel-status">
          <span className={isRecording ? "status-light live" : "status-light"} />
          <span>{formatDuration(elapsedMs)}</span>
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
          <span>{transcriptionStatus?.message ?? "Checking local transcription…"}</span>
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

      <NoticeBar notice={notice} />

      {selectedThread && hasSegments && (
        <TranscriptToolbar
          query={query}
          speakers={speakers}
          speakerLabels={selectedThread.speakerLabels}
          canModify={canModify}
          onQueryChange={setQuery}
          onCopy={() => void threadActions.copyTranscript()}
          onExport={() => void threadActions.exportMarkdown()}
          onReveal={() => void threadActions.revealPath(selectedThread.summary.path)}
          onDelete={() => void threadActions.deleteThread()}
          onRenameSpeaker={(speaker, label) => void threadActions.renameSpeaker(speaker, label)}
        />
      )}

      <TranscriptSurface
        recorderState={recorderState}
        selectedThread={selectedThread}
        query={query}
        onCreateThread={onCreateThread}
        onStartRecording={onStartRecording}
        onSaveSegmentText={(index, text) => void threadActions.updateSegmentText(index, text)}
      />
      <PanelFooter
        liveStatus={liveStatus}
        finalization={finalization}
        selectedThread={selectedThread}
        onRevealMarkdown={(path) => void threadActions.revealPath(path)}
      />
      <ErrorToast message={error} />
    </section>
  );
}
