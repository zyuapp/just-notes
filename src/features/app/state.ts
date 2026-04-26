import type { AppInfo } from "../../bindings/AppInfo";
import type { LiveTranscriptSegmentPayload } from "../../bindings/LiveTranscriptSegmentPayload";
import type { LiveTranscriptStatusPayload } from "../../bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "../../bindings/MeterPayload";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { ThreadDetail } from "../../bindings/ThreadDetail";
import type { ThreadSummary } from "../../bindings/ThreadSummary";
import type { TranscriptionStatusPayload } from "../../bindings/TranscriptionStatusPayload";
import { applyLiveSegmentToThread, applyLiveSegmentToThreadList } from "../../lib/transcript";

export type RecorderState = "idle" | "starting" | "recording" | "stopping";

export type AppState = {
  appInfo: AppInfo | null;
  threads: ThreadSummary[];
  selectedThreadId: string | null;
  selectedThread: ThreadDetail | null;
  recorderState: RecorderState;
  meters: MeterPayload;
  liveStatus: LiveTranscriptStatusPayload | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  error: string | null;
};

export const emptyMeters: MeterPayload = {
  threadId: "",
  micLevel: 0,
  systemLevel: 0,
  elapsedMs: 0,
};

export const initialAppState: AppState = {
  appInfo: null,
  threads: [],
  selectedThreadId: null,
  selectedThread: null,
  recorderState: "idle",
  meters: emptyMeters,
  liveStatus: null,
  transcriptionStatus: null,
  error: null,
};

export type AppAction =
  | { type: "errorCleared" }
  | { type: "failed"; message: string }
  | {
      type: "bootstrapLoaded";
      info: AppInfo;
      transcriptionStatus: TranscriptionStatusPayload;
      threads: ThreadSummary[];
    }
  | { type: "threadsLoaded"; threads: ThreadSummary[] }
  | { type: "threadSelected"; detail: ThreadDetail }
  | { type: "recordingStarting" }
  | { type: "recordingStarted"; payload: RecordingPayload }
  | { type: "recordingStartFailed"; message: string }
  | { type: "recordingStopping" }
  | { type: "recordingStopped"; detail: ThreadDetail }
  | { type: "recordingStopFailed"; message: string }
  | { type: "meterReceived"; payload: MeterPayload }
  | { type: "liveSegmentReceived"; payload: LiveTranscriptSegmentPayload; updatedAtMs: number }
  | { type: "liveStatusReceived"; payload: LiveTranscriptStatusPayload }
  | { type: "liveErrorReceived"; payload: LiveTranscriptStatusPayload };

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
      };
    case "threadsLoaded":
      return { ...state, threads: action.threads };
    case "threadSelected":
      return {
        ...state,
        selectedThreadId: action.detail.summary.id,
        selectedThread: action.detail,
      };
    case "recordingStarting":
      return { ...state, error: null, recorderState: "starting", meters: emptyMeters };
    case "recordingStarted":
      return {
        ...state,
        selectedThreadId: action.payload.thread.summary.id,
        selectedThread: action.payload.thread,
        transcriptionStatus: action.payload.transcription,
        recorderState: "recording",
      };
    case "recordingStartFailed":
      return { ...state, error: action.message, recorderState: "idle" };
    case "recordingStopping":
      return { ...state, error: null, recorderState: "stopping" };
    case "recordingStopped":
      return {
        ...state,
        selectedThreadId: action.detail.summary.id,
        selectedThread: action.detail,
        meters: { ...state.meters, micLevel: 0, systemLevel: 0 },
        recorderState: "idle",
      };
    case "recordingStopFailed":
      return { ...state, error: action.message, recorderState: "recording" };
    case "meterReceived":
      return { ...state, meters: action.payload };
    case "liveSegmentReceived":
      return {
        ...state,
        selectedThread: applyLiveSegmentToThread(
          state.selectedThread,
          action.payload,
          action.updatedAtMs,
        ),
        threads: applyLiveSegmentToThreadList(state.threads, action.payload, action.updatedAtMs),
      };
    case "liveStatusReceived":
      return { ...state, liveStatus: action.payload };
    case "liveErrorReceived":
      return { ...state, liveStatus: action.payload, error: action.payload.message };
  }
}

export function getActiveThreadId(state: AppState) {
  return state.liveStatus?.active ? state.liveStatus.threadId : null;
}

export function getStatusLabel(state: AppState) {
  if (state.recorderState === "starting") return "Starting";
  if (state.recorderState === "recording") return "Recording";
  if (state.recorderState === "stopping") return "Stopping";
  return "Ready";
}
