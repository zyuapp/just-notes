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
        meetingAccess: action.meetingAccess,
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
      };
    }
    case "archiveOpenChanged":
      return { ...state, archiveOpen: action.open };
    case "settingsLoaded":
      return { ...state, settings: action.settings };
    case "transcriptionStatusLoaded":
      return { ...state, transcriptionStatus: action.transcriptionStatus };
    case "permissionsLoaded":
      return { ...state, permissions: action.permissions };
    case "meetingAccessLoaded":
      return { ...state, meetingAccess: action.meetingAccess };
    case "meetingPromptChanged":
      return { ...state, meetingPrompt: action.meetingPrompt };
    case "settingsOpenChanged":
      return { ...state, settingsOpen: action.open };
    case "finalizationReceived":
      return { ...state, finalization: action.payload };
    case "recordingStarting":
      return {
        ...state,
        error: null,
        recorderState: "starting",
        meters: emptyMeters,
        liveSegments: [],
        finalization: null,
      };
    case "recordingStarted":
      return {
        ...state,
        selectedThreadId: action.payload.thread.summary.id,
        recordingThreadId: action.payload.thread.summary.id,
        selectedThread: action.payload.thread,
        transcriptionStatus: action.payload.transcription,
        recorderState: "recording",
        meetingPrompt: null,
      };
    case "recordingStartFailed":
      if (state.recorderState !== "starting") return state;
      return { ...state, error: action.message, recorderState: "idle", recordingThreadId: null };
    case "recordingStopping":
      return { ...state, error: null, recorderState: "stopping" };
    case "recordingStopped":
      // The detail already carries the live-persisted transcript, so the preview
      // is now redundant — swap to the authoritative thread and drop it.
      return {
        ...state,
        selectedThreadId: action.detail.summary.id,
        selectedThread: action.detail,
        threads: replaceThreadSummary(state, action.detail),
        meters: emptyMeters,
        liveSegments: [],
        recordingThreadId: null,
        recorderState: "idle",
      };
    case "recordingStopFailed":
      return { ...state, error: action.message, recorderState: "recording" };
    case "meterReceived":
      return { ...state, meters: action.payload };
    case "liveSegmentReceived":
      // Ignore a late event delivered after stop, and any not for the thread
      // currently being recorded.
      if (state.recorderState !== "recording" && state.recorderState !== "stopping") {
        return state;
      }
      if (state.recordingThreadId !== action.payload.threadId) {
        return state;
      }
      return { ...state, liveSegments: [...state.liveSegments, action.payload.segment] };
  }
}

function replaceThreadSummary(state: AppState, detail: ThreadDetail) {
  return state.threads.map((thread) =>
    thread.id === detail.summary.id ? detail.summary : thread,
  );
}
