import { type CSSProperties, useCallback, useMemo, useReducer } from "react";
import { ArchivedView } from "./components/ArchivedView";
import { AppSettingsOverlay } from "./components/AppSettingsOverlay";
import { LegacyImportDialog } from "./components/LegacyImportDialog";
import { ModelDownloadDialog } from "./components/ModelDownloadDialog";
import { ThreadSidebar } from "./components/ThreadSidebar";
import { TranscriptPanel } from "./components/TranscriptPanel";
import { useArchiveFlight } from "./components/useArchiveFlight";
import { buildNotice } from "./features/app/buildNotice";
import { openLegacyImportWorkflow } from "./features/app/legacyImportOpening";
import { appReducer, getActiveThreadId, getStatusLabel, initialAppState } from "./features/app/state";
import { useAppEvents } from "./features/app/useAppEvents";
import { useArchivedThreads } from "./features/app/useArchivedThreads";
import { useJustNotesController } from "./features/app/useJustNotesController";
import { useLegacyImportController } from "./features/app/useLegacyImportController";
import { useMeetingSettingsController } from "./features/app/useMeetingSettingsController";
import { useMeetingPromptController } from "./features/app/useMeetingPromptController";
import { useSettingsController } from "./features/app/useSettingsController";
import { useSidebarWidth } from "./features/app/useSidebarWidth";
import { useThreadActions } from "./features/app/useThreadActions";
import { useThreadSearch } from "./features/app/useThreadSearch";
import { MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH } from "./lib/sidebarWidth";
import { getApiErrorMessage } from "./api";
export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const actions = useJustNotesController(state, dispatch);
  const threadActions = useThreadActions(state, dispatch, actions.refreshThreads);
  const settingsActions = useSettingsController(state, dispatch, actions.bootstrap);
  const legacyImport = useLegacyImportController(
    useCallback(
      (error: unknown) => dispatch({ type: "failed", message: getApiErrorMessage(error) }),
      [],
    ),
    actions.bootstrap,
  );
  const meetingSettingsActions = useMeetingSettingsController(state, dispatch, settingsActions.updateSettings);
  const meetingPromptActions = useMeetingPromptController(dispatch, actions.refreshThreads);
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
  const onFinalizationSettled = useCallback(() => void actions.refreshThreads(), [actions.refreshThreads]);
  const onRecordingStarted = useCallback(
    (threadId: string) => void actions.refreshThreads(threadId), [actions.refreshThreads],
  );
  const openLegacyImport = useCallback(() => {
    openLegacyImportWorkflow(settingsActions.closeSettings, legacyImport.open);
  }, [legacyImport.open, settingsActions.closeSettings]);
  useAppEvents(dispatch, onFinalizationSettled, onRecordingStarted, openLegacyImport);
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
        finalization={state.finalization}
        meters={state.meters}
        notice={notice}
        recorderState={state.recorderState}
        selectedThread={state.selectedThread}
        liveSegments={state.selectedThreadId === state.recordingThreadId ? state.liveSegments : []}
        meetingPrompt={state.meetingPrompt}
        statusLabel={statusLabel}
        transcriptionStatus={state.transcriptionStatus}
        threadActions={threadActions}
        onArchiveThread={archiveThread}
        onReprocess={(threadId) => void actions.reprocessThread(threadId)}
        onCancelModelDownload={actions.cancelModelDownload}
        onStartModelDownload={actions.startModelDownload}
        onStartFixtureRecording={actions.startFixtureRecording}
        onStartRecording={actions.startRecording}
        onStartMeetingRecording={(id) => void meetingPromptActions.startMeetingRecording(id)}
        onDismissMeetingPrompt={(id) => void meetingPromptActions.dismissMeetingPrompt(id)}
        onStopRecording={actions.stopRecording}
      />
      <AppSettingsOverlay state={state} actions={actions} meetingSettings={meetingSettingsActions}
        settings={settingsActions} threads={threadActions} onImportLegacyData={openLegacyImport} />
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
      {legacyImport.state && (
        <LegacyImportDialog
          state={legacyImport.state}
          onClose={legacyImport.close}
          onLocate={legacyImport.locate}
          onConfirm={legacyImport.confirm}
          onViewImported={legacyImport.close}
        />
      )}
    </main>
  );
}
