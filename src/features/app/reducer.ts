import type { ThreadDetail } from "../../bindings/ThreadDetail";
import { emptyMeters, type AppAction, type AppState } from "./state";

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "errorCleared":
      return { ...state, error: null };
    case "failed":
      return { ...state, error: action.message };
    case "bootstrapLoaded":
      return {
        ...state,
        appInfo: action.info,
        transcriptionStatus: action.transcriptionStatus,
        threads: action.threads,
        settings: action.settings,
        permissions: action.permissions,
      };
    case "threadsLoaded":
      return { ...state, threads: action.threads };
    case "threadSelected":
    case "threadUpdated":
      return {
        ...state,
        selectedThreadId: action.detail.summary.id,
        selectedThread: action.detail,
        threads: replaceThreadSummary(state, action.detail),
      };
    case "threadArchived": {
      const threadId = action.summary.id;
      return {
        ...state,
        threads: state.threads.filter((thread) => thread.id !== threadId),
        selectedThreadId: state.selectedThreadId === threadId ? null : state.selectedThreadId,
        recordingThreadId:
          state.recordingThreadId === threadId ? null : state.recordingThreadId,
        selectedThread:
          state.selectedThread?.summary.id === threadId ? null : state.selectedThread,
        archivedNotice: { threadId, title: action.summary.title },
      };
    }
    case "archiveNoticeCleared":
      // A targeted clear (from restore/permanent-delete of a specific thread)
      // must compare against current state, not a value captured in a stale
      // closure; an untargeted clear (the auto-dismiss timer) always clears.
      if (action.threadId && state.archivedNotice?.threadId !== action.threadId) return state;
      return { ...state, archivedNotice: null };
    case "archiveOpenChanged":
      // Opening the archive view retires the undo affordance: the user is now
      // managing archived threads directly, so the toast would be redundant.
      return {
        ...state,
        archiveOpen: action.open,
        archivedNotice: action.open ? null : state.archivedNotice,
      };
    case "settingsLoaded":
      return { ...state, settings: action.settings };
    case "transcriptionStatusLoaded":
      return { ...state, transcriptionStatus: action.transcriptionStatus };
    case "permissionsLoaded":
      return { ...state, permissions: action.permissions };
    case "settingsOpenChanged":
      return { ...state, settingsOpen: action.open };
    case "finalizationReceived":
      return { ...state, finalization: action.payload };
    case "recordingStarting":
      return { ...state, error: null, recorderState: "starting", meters: emptyMeters };
    case "recordingStarted":
      return {
        ...state,
        selectedThreadId: action.payload.thread.summary.id,
        recordingThreadId: action.payload.thread.summary.id,
        selectedThread: action.payload.thread,
        transcriptionStatus: action.payload.transcription,
        recorderState: "recording",
      };
    case "recordingStartFailed":
      return { ...state, error: action.message, recorderState: "idle", recordingThreadId: null };
    case "recordingStopping":
      return { ...state, error: null, recorderState: "stopping" };
    case "recordingStopped":
      return {
        ...state,
        selectedThreadId: action.detail.summary.id,
        selectedThread: action.detail,
        threads: replaceThreadSummary(state, action.detail),
        meters: emptyMeters,
        recordingThreadId: null,
        recorderState: "idle",
      };
    case "recordingStopFailed":
      return { ...state, error: action.message, recorderState: "recording" };
    case "meterReceived":
      return { ...state, meters: action.payload };
  }
}

function replaceThreadSummary(state: AppState, detail: ThreadDetail) {
  return state.threads.map((thread) =>
    thread.id === detail.summary.id ? detail.summary : thread,
  );
}
