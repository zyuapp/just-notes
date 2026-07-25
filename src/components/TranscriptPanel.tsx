import { useState } from "react";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { MeetingPromptPayload } from "../bindings/MeetingPromptPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";
import type { RecordButtonControl } from "../features/app/recordButtonState";
import type { ThreadActions } from "../features/app/useThreadActions";
import { formatCompactDuration, formatThreadDate } from "../lib/format";
import { CaptureBar } from "./CaptureBar";
import { ErrorToast } from "./ErrorToast";
import { NoticeBar, type Notice } from "./NoticeBar";
import { MeetingPromptCard } from "./MeetingPromptCard";
import { ThreadTitle } from "./ThreadTitle";
import { TranscriptSurface } from "./TranscriptSurface";
import { TranscriptToolbar } from "./TranscriptToolbar";

type TranscriptPanelProps = {
  error: string | null;
  fixtureMode: boolean;
  meters: MeterPayload;
  notice: Notice | null;
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  liveSegments: TranscriptSegment[];
  meetingPrompt: MeetingPromptPayload | null;
  recordButtonControl: RecordButtonControl;
  transcriptionStatus: TranscriptionStatusPayload | null;
  threadActions: ThreadActions;
  onArchiveThread: (threadId: string) => void;
  onStartFixtureRecording: () => void;
  onStartMeetingRecording: (requestId: string) => void;
  onDismissMeetingPrompt: (requestId: string) => void;
};

export function TranscriptPanel({
  error,
  fixtureMode,
  meters,
  notice,
  recorderState,
  selectedThread,
  liveSegments,
  meetingPrompt,
  recordButtonControl,
  transcriptionStatus,
  threadActions,
  onArchiveThread,
  onStartFixtureRecording,
  onStartMeetingRecording,
  onDismissMeetingPrompt,
}: TranscriptPanelProps) {
  const [query, setQuery] = useState("");
  const isRecording = recorderState === "recording";
  const summary = selectedThread?.summary ?? null;
  const hasSegments = (selectedThread?.segments.length ?? 0) > 0;
  const canModify = recorderState === "idle" && summary?.status === "idle";

  return (
    <section className="thread-panel" aria-label="Transcript">
      <header className="panel-head" data-tauri-drag-region="">
        <ThreadTitle
          title={summary?.title ?? null}
          canRename={canModify}
          onRename={(title) => void threadActions.renameThread(title)}
        />
        <p className="eyebrow">
          <span className={isRecording ? "status-dot live" : "status-dot"} />
          {summary ? (
            <>
              <span>{formatThreadDate(summary.createdAtMs)}</span>
              {summary.durationMs > 0 && (
                <>
                  <i>·</i>
                  <time>{formatCompactDuration(summary.durationMs)}</time>
                </>
              )}
            </>
          ) : (
            <span>Local</span>
          )}
        </p>
      </header>

      {selectedThread && hasSegments && (
        <TranscriptToolbar
          query={query}
          canModify={canModify}
          onQueryChange={setQuery}
          onCopy={() => void threadActions.copyTranscript()}
          onArchive={() => onArchiveThread(selectedThread.summary.id)}
        />
      )}

      <NoticeBar notice={notice} />

      {recorderState === "idle" && meetingPrompt && (
        <MeetingPromptCard
          prompt={meetingPrompt}
          onStart={() => onStartMeetingRecording(meetingPrompt.requestId)}
          onDismiss={() => onDismissMeetingPrompt(meetingPrompt.requestId)}
        />
      )}

      <TranscriptSurface
        recorderState={recorderState}
        selectedThread={selectedThread}
        liveSegments={liveSegments}
        transcriptionStatus={transcriptionStatus}
        query={query}
      />

      <CaptureBar
        recorderState={recorderState}
        meters={meters}
        transcriptionStatus={transcriptionStatus}
        recordButtonControl={recordButtonControl}
        fixtureMode={fixtureMode}
        onStartFixtureRecording={onStartFixtureRecording}
      />
      <ErrorToast message={error} />
    </section>
  );
}
