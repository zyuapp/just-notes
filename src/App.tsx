import { useCallback, useMemo, useReducer } from "react";
import { ArchivedView } from "./components/ArchivedView";
import { SettingsView } from "./components/SettingsView";
import { ThreadSidebar } from "./components/ThreadSidebar";
import { TranscriptPanel } from "./components/TranscriptPanel";
import { buildNotice } from "./features/app/buildNotice";
import { appReducer, getActiveThreadId, getStatusLabel, initialAppState } from "./features/app/state";
import { useAppEvents } from "./features/app/useAppEvents";
import { useArchivedThreads } from "./features/app/useArchivedThreads";
import { useJustNotesController } from "./features/app/useJustNotesController";
import { useSettingsController } from "./features/app/useSettingsController";
import { useThreadActions } from "./features/app/useThreadActions";
import { useThreadSearch } from "./features/app/useThreadSearch";

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const actions = useJustNotesController(state, dispatch);
  const threadActions = useThreadActions(state, dispatch, actions.refreshThreads);
  const settingsActions = useSettingsController(state, dispatch, actions.bootstrap);
  const search = useThreadSearch(dispatch, state.threads);
  const archived = useArchivedThreads(state.archiveOpen);

  const onFinalizationSettled = useCallback(
    () => void actions.refreshThreads(),
    [actions.refreshThreads],
  );
  useAppEvents(dispatch, onFinalizationSettled);

  const activeThreadId = getActiveThreadId(state);
  const statusLabel = useMemo(() => getStatusLabel(state), [state]);
  const notice = useMemo(
    () =>
      buildNotice(
        state,
        settingsActions.openSettings,
        settingsActions.openPrivacySettings,
        actions.startModelDownload,
      ),
    [
      actions.startModelDownload,
      state,
      settingsActions.openPrivacySettings,
      settingsActions.openSettings,
    ],
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
        onExportThread={(threadId) => void threadActions.exportMarkdown(threadId)}
        onRevealThread={(path) => void threadActions.revealPath(path)}
        onArchiveThread={(threadId) => void threadActions.archiveThread(threadId)}
        onOpenArchive={() => dispatch({ type: "archiveOpenChanged", open: true })}
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
        onCancelModelDownload={actions.cancelModelDownload}
        onStartModelDownload={actions.startModelDownload}
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
          onCancelModelDownload={() => void actions.cancelModelDownload()}
          onStartModelDownload={() => void actions.startModelDownload()}
          onOpenPrivacy={(pane) => void settingsActions.openPrivacySettings(pane)}
        />
      )}
      {state.archiveOpen && (
        <ArchivedView
          items={archived.items}
          error={archived.error}
          onReload={archived.reload}
          onClose={() => dispatch({ type: "archiveOpenChanged", open: false })}
          onRestore={(threadId) => threadActions.restoreThread(threadId)}
          onDeletePermanently={(threadId) => threadActions.deleteArchivedThread(threadId)}
        />
      )}
    </main>
  );
}
