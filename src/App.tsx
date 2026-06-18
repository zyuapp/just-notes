import { useCallback, useMemo, useReducer } from "react";
import type { Notice } from "./components/NoticeBar";
import { SettingsView } from "./components/SettingsView";
import { ThreadSidebar } from "./components/ThreadSidebar";
import { TranscriptPanel } from "./components/TranscriptPanel";
import {
  appReducer,
  getActiveThreadId,
  getStatusLabel,
  initialAppState,
  type AppState,
} from "./features/app/state";
import { useAppEvents } from "./features/app/useAppEvents";
import { useJustNotesController } from "./features/app/useJustNotesController";
import { useSettingsController } from "./features/app/useSettingsController";
import { useThreadActions } from "./features/app/useThreadActions";
import { useThreadSearch } from "./features/app/useThreadSearch";

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const actions = useJustNotesController(state, dispatch);
  const threadActions = useThreadActions(state, dispatch, actions.refreshThreads);
  const settingsActions = useSettingsController(state, dispatch, actions.bootstrap);
  const search = useThreadSearch(dispatch);

  const onFinalizationSettled = useCallback(
    () => void actions.refreshThreads(),
    [actions.refreshThreads],
  );
  useAppEvents(dispatch, onFinalizationSettled);

  const activeThreadId = getActiveThreadId(state);
  const statusLabel = useMemo(() => getStatusLabel(state), [state]);
  const notice = useMemo(
    () => buildNotice(state, settingsActions.openSettings, settingsActions.openPrivacySettings),
    [state, settingsActions.openSettings, settingsActions.openPrivacySettings],
  );

  return (
    <main className="app-shell">
      <ThreadSidebar
        activeThreadId={activeThreadId}
        appInfo={state.appInfo}
        selectedThreadId={state.selectedThreadId}
        threads={search.results ?? state.threads}
        searchQuery={search.query}
        searching={search.results !== null}
        onSearchChange={search.setQuery}
        onCreateThread={actions.createThread}
        onSelectThread={(threadId) => void actions.selectThread(threadId)}
        onOpenSettings={settingsActions.openSettings}
        onRevealStorage={() => {
          if (state.appInfo) void threadActions.revealPath(state.appInfo.threadsDir);
        }}
      />
      <TranscriptPanel
        error={state.error}
        fixtureMode={state.appInfo?.fixtureMode ?? false}
        finalization={state.finalization}
        meters={state.meters}
        notice={notice}
        recorderState={state.recorderState}
        selectedThread={state.selectedThread}
        statusLabel={statusLabel}
        transcriptionStatus={state.transcriptionStatus}
        threadActions={threadActions}
        onCreateThread={actions.createThread}
        onStartFixtureRecording={actions.startFixtureRecording}
        onStartRecording={actions.startRecording}
        onStopRecording={actions.stopRecording}
      />
      {state.settingsOpen && state.settings && (
        <SettingsView
          settings={state.settings}
          appInfo={state.appInfo}
          transcriptionStatus={state.transcriptionStatus}
          permissions={state.permissions}
          onClose={settingsActions.closeSettings}
          onChooseFolder={() => void settingsActions.chooseTranscriptsFolder()}
          onUseDefaultFolder={() => void settingsActions.useDefaultFolder()}
          onRevealFolder={() => {
            if (state.appInfo) void threadActions.revealPath(state.appInfo.threadsDir);
          }}
          onToggleRawAudio={() => void settingsActions.toggleRawAudio()}
          onToggleMarkdownCopy={() => void settingsActions.toggleMarkdownCopy()}
          onOpenPrivacy={(pane) => void settingsActions.openPrivacySettings(pane)}
        />
      )}
    </main>
  );
}

function buildNotice(
  state: AppState,
  openSettings: () => void,
  openPrivacy: (pane: "microphone" | "system-audio") => Promise<void>,
): Notice | null {
  if (state.permissions && ["denied", "restricted"].includes(state.permissions.microphone)) {
    return {
      message: "Microphone access is blocked, so recordings will miss your voice.",
      actionLabel: "Open System Settings",
      onAction: () => void openPrivacy("microphone"),
    };
  }
  if (state.transcriptionStatus && !state.transcriptionStatus.ready) {
    return {
      message:
        "The local transcription model is not installed yet, so recordings will capture audio without a transcript.",
      actionLabel: "Model status",
      onAction: openSettings,
    };
  }
  return null;
}
