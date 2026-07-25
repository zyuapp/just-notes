import { type CSSProperties, useCallback, useMemo, useReducer } from "react";
import { ArchivedView } from "./components/ArchivedView";
import { AppSettingsOverlay } from "./components/AppSettingsOverlay";
import { ModelDownloadDialog } from "./components/ModelDownloadDialog";
import { ThreadSidebar } from "./components/ThreadSidebar";
import { TranscriptPanel } from "./components/TranscriptPanel";
import { useArchiveFlight } from "./components/useArchiveFlight";
import { buildNotice } from "./features/app/buildNotice";
import { buildRecordButtonControl } from "./features/app/recordButtonState";
import { appReducer, getActiveThreadId, initialAppState } from "./features/app/state";
import { useAppEvents } from "./features/app/useAppEvents";
import { useArchivedThreads } from "./features/app/useArchivedThreads";
import { useJustNotesController } from "./features/app/useJustNotesController";
import { useMeetingSettingsController } from "./features/app/useMeetingSettingsController";
import { useMeetingPromptController } from "./features/app/useMeetingPromptController";
import { useSettingsController } from "./features/app/useSettingsController";
import { useSettingsShortcut } from "./features/app/useSettingsShortcut";
import { useSidebarWidth } from "./features/app/useSidebarWidth";
import { useStorageController } from "./features/app/useStorageController";
import { useThreadActions } from "./features/app/useThreadActions";
import { useThreadSearch } from "./features/app/useThreadSearch";
import { useWindowFocusRefresh } from "./features/app/useWindowFocusRefresh";
import { MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH } from "./lib/sidebarWidth";
export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const actions = useJustNotesController(state, dispatch);
  const threadActions = useThreadActions(state, dispatch, actions.refreshThreads);
  const settingsActions = useSettingsController(state, dispatch, actions.bootstrap);
  const meetingSettingsActions = useMeetingSettingsController(state, dispatch, settingsActions.updateSettings);
  const meetingPromptActions = useMeetingPromptController(dispatch, actions.refreshThreads);
  const storageActions = useStorageController(dispatch);
  const refreshSettingsData = useCallback(() => {
    settingsActions.refreshPermissions();
    storageActions.refreshUsage();
  }, [settingsActions.refreshPermissions, storageActions.refreshUsage]);
  useWindowFocusRefresh(state.settingsOpen, refreshSettingsData);
  useSettingsShortcut(settingsActions.openSettings);
  const search = useThreadSearch(dispatch, state.threads);
  const archived = useArchivedThreads(state.archiveOpen);
  const { iconRef, scopeRef, flyToArchive } = useArchiveFlight();
  const { width: sidebarWidth, onResizeStart, onResizeKeyDown, resetWidth } = useSidebarWidth();
  const archiveThread = useCallback(
    (threadId: string) => {
      flyToArchive(threadId);
      void threadActions.archiveThread(threadId);
    },
    [flyToArchive, threadActions],
  );
  const onRecordingStarted = useCallback(
    (threadId: string) => void actions.refreshThreads(threadId), [actions.refreshThreads],
  );
  useAppEvents(dispatch, onRecordingStarted);
  const activeThreadId = getActiveThreadId(state);
  const recordButtonControl = buildRecordButtonControl(state, actions);
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
    <main className="app-shell" style={{ "--sidebar-width": `${sidebarWidth}px` } as CSSProperties}>
      <ThreadSidebar
        activeThreadId={activeThreadId}
        selectedThreadId={state.selectedThreadId}
        threads={search.results ?? state.threads}
        searchQuery={search.query}
        searching={search.results !== null}
        iconRef={iconRef}
        scopeRef={scopeRef}
        onSearchChange={search.setQuery}
        onCreateThread={actions.createThread}
        onSelectThread={(threadId) => void actions.selectThread(threadId)}
        onExportThread={(threadId) => void threadActions.exportMarkdown(threadId)}
        onRevealThread={(path) => void threadActions.revealPath(path)}
        onArchiveThread={archiveThread}
        onOpenArchive={() => dispatch({ type: "archiveOpenChanged", open: true })}
        onOpenSettings={settingsActions.openSettings}
      />
      <div
        className="sidebar-resizer"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        aria-valuenow={sidebarWidth}
        aria-valuemin={MIN_SIDEBAR_WIDTH}
        aria-valuemax={MAX_SIDEBAR_WIDTH}
        tabIndex={0}
        onPointerDown={onResizeStart}
        onKeyDown={onResizeKeyDown}
        onDoubleClick={resetWidth}
      />
      <TranscriptPanel
        error={state.error}
        fixtureMode={state.appInfo?.fixtureMode ?? false}
        meters={state.meters}
        notice={notice}
        recorderState={state.recorderState}
        selectedThread={state.selectedThread}
        liveSegments={state.selectedThreadId === state.recordingThreadId ? state.liveSegments : []}
        meetingPrompt={state.meetingPrompt}
        recordButtonControl={recordButtonControl}
        transcriptionStatus={state.transcriptionStatus}
        threadActions={threadActions}
        onArchiveThread={archiveThread}
        onStartFixtureRecording={actions.startFixtureRecording}
        onStartMeetingRecording={(id) => void meetingPromptActions.startMeetingRecording(id)}
        onDismissMeetingPrompt={(id) => void meetingPromptActions.dismissMeetingPrompt(id)}
      />
      <AppSettingsOverlay state={state} actions={actions} meetingSettings={meetingSettingsActions}
        settings={settingsActions} storage={storageActions} threads={threadActions} />
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
      {actions.modelDownloadConsent && (
        <ModelDownloadDialog model={actions.modelDownloadConsent} onCancel={actions.dismissModelDownloadConsent}
          onConfirm={() => void actions.confirmModelDownload()} />
      )}
    </main>
  );
}
