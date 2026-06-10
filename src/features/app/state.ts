import type { AppInfo } from "../../bindings/AppInfo";
import type { AppSettings } from "../../bindings/AppSettings";
import type { FinalizationStatusPayload } from "../../bindings/FinalizationStatusPayload";
import type { LiveTranscriptSegmentPayload } from "../../bindings/LiveTranscriptSegmentPayload";
import type { LiveTranscriptStatusPayload } from "../../bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "../../bindings/MeterPayload";
import type { PermissionsPayload } from "../../bindings/PermissionsPayload";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { ThreadDetail } from "../../bindings/ThreadDetail";
import type { ThreadSummary } from "../../bindings/ThreadSummary";
import type { TranscriptionStatusPayload } from "../../bindings/TranscriptionStatusPayload";

export { appReducer } from "./reducer";

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
  settings: AppSettings | null;
  permissions: PermissionsPayload | null;
  finalization: FinalizationStatusPayload | null;
  settingsOpen: boolean;
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
  settings: null,
  permissions: null,
  finalization: null,
  settingsOpen: false,
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
      settings: AppSettings;
      permissions: PermissionsPayload;
    }
  | { type: "threadsLoaded"; threads: ThreadSummary[] }
  | { type: "threadSelected"; detail: ThreadDetail }
  | { type: "threadUpdated"; detail: ThreadDetail }
  | { type: "threadDeleted"; threadId: string }
  | { type: "settingsLoaded"; settings: AppSettings }
  | { type: "permissionsLoaded"; permissions: PermissionsPayload }
  | { type: "settingsOpenChanged"; open: boolean }
  | { type: "finalizationReceived"; payload: FinalizationStatusPayload }
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

export function getActiveThreadId(state: AppState) {
  return state.liveStatus?.active ? state.liveStatus.threadId : null;
}

export function getStatusLabel(state: AppState) {
  if (state.recorderState === "starting") return "Starting";
  if (state.recorderState === "recording") return "Recording";
  if (state.recorderState === "stopping") return "Stopping";
  return "Ready";
}
