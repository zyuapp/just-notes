import { useState } from "react";
import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import type { ThreadActions } from "../features/app/useThreadActions";
import { formatDuration, formatThreadDate } from "../lib/format";
import { CaptureBar } from "./CaptureBar";
import { ErrorToast } from "./ErrorToast";
import { NoticeBar, type Notice } from "./NoticeBar";
import { ThreadTitle } from "./ThreadTitle";
import { TranscriptSurface } from "./TranscriptSurface";
import { TranscriptToolbar } from "./TranscriptToolbar";

type TranscriptPanelProps = {
  error: string | null;
  fixtureMode: boolean;
  finalization: FinalizationStatusPayload | null;
  meters: MeterPayload;
  notice: Notice | null;
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  statusLabel: string;
  transcriptionStatus: TranscriptionStatusPayload | null;
  threadActions: ThreadActions;
  onCancelModelDownload: () => void;
  onStartModelDownload: () => void;
  onStartFixtureRecording: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
};

export function TranscriptPanel({
  error,
  fixtureMode,
  finalization,
  meters,
  notice,
  recorderState,
  selectedThread,
  statusLabel,
  transcriptionStatus,
  threadActions,
  onCancelModelDownload,
  onStartModelDownload,
  onStartFixtureRecording,
  onStartRecording,
  onStopRecording,
}: TranscriptPanelProps) {
  const [query, setQuery] = useState("");
  const isRecording = recorderState === "recording";
  const summary = selectedThread?.summary ?? null;
  const hasSegments = (selectedThread?.segments.length ?? 0) > 0;
  const canModify = recorderState === "idle" && summary?.status === "idle";
  const canResume = canModify && hasSegments;
  const speakers = Array.from(
    new Set(selectedThread?.segments.map((segment) => segment.speaker) ?? []),
  );

  return (
    <section className="thread-panel" aria-label="Transcript">
      <header className="panel-head" data-tauri-drag-region="">
        <p className="eyebrow">
          <span className={isRecording ? "status-dot live" : "status-dot"} />
          {summary ? (
            <>
              <span>{formatThreadDate(summary.createdAtMs)}</span>
              {summary.durationMs > 0 && (
                <>
                  <i>·</i>
                  <time>{formatDuration(summary.durationMs)}</time>
                </>
              )}
              {(hasSegments || summary.hasAudio) && (
                <>
                  <i>·</i>
                  <span>Mic + System</span>
                </>
              )}
            </>
          ) : (
            <span>Local</span>
          )}
        </p>
        <ThreadTitle
          title={summary?.title ?? null}
          canRename={canModify}
          onRename={(title) => void threadActions.renameThread(title)}
        />
      </header>

      {selectedThread && hasSegments && (
        <TranscriptToolbar
          query={query}
          speakers={speakers}
          speakerLabels={selectedThread.speakerLabels}
          canModify={canModify}
          onQueryChange={setQuery}
          onCopy={() => void threadActions.copyTranscript()}
          onExport={() => void threadActions.exportMarkdown(selectedThread.summary.id)}
          onReveal={() => void threadActions.revealPath(selectedThread.summary.path)}
          onArchive={() => void threadActions.archiveThread(selectedThread.summary.id)}
          onRenameSpeaker={(speaker, label) => void threadActions.renameSpeaker(speaker, label)}
        />
      )}

      <NoticeBar notice={notice} />

      <TranscriptSurface
        recorderState={recorderState}
        selectedThread={selectedThread}
        transcriptionStatus={transcriptionStatus}
        query={query}
        onSaveSegmentText={(index, text) => void threadActions.updateSegmentText(index, text)}
      />

      <CaptureBar
        recorderState={recorderState}
        meters={meters}
        finalization={finalization}
        transcriptionStatus={transcriptionStatus}
        selectedThreadId={summary?.id ?? null}
        resumeSelected={canResume}
        statusLabel={statusLabel}
        fixtureMode={fixtureMode}
        onCancelModelDownload={onCancelModelDownload}
        onStartModelDownload={onStartModelDownload}
        onStartRecording={onStartRecording}
        onStopRecording={onStopRecording}
        onStartFixtureRecording={onStartFixtureRecording}
      />
      <ErrorToast message={error} />
    </section>
  );
}
