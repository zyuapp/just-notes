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
    case "threadDeleted":
      return {
        ...state,
        threads: state.threads.filter((thread) => thread.id !== action.threadId),
        selectedThreadId: state.selectedThreadId === action.threadId ? null : state.selectedThreadId,
        recordingThreadId:
          state.recordingThreadId === action.threadId ? null : state.recordingThreadId,
        selectedThread:
          state.selectedThread?.summary.id === action.threadId ? null : state.selectedThread,
      };
    case "settingsLoaded":
      return { ...state, settings: action.settings };
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
