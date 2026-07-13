import { listen } from "@tauri-apps/api/event";
import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { LiveTranscriptPayload } from "../bindings/LiveTranscriptPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { MeetingPromptPayload } from "../bindings/MeetingPromptPayload";
import type { RecordingPayload } from "../bindings/RecordingPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import { toApiError } from "./errors";

export type UnlistenFn = () => void;

type EventHandler<T> = (payload: T) => void;

async function listenToEvent<T>(eventName: string, handler: EventHandler<T>): Promise<UnlistenFn> {
  try {
    return await listen<T>(eventName, (event) => {
      handler(event.payload);
    });
  } catch (error) {
    throw toApiError(error);
  }
}

export const eventsApi = {
  onMeter(handler: EventHandler<MeterPayload>): Promise<UnlistenFn> {
    return listenToEvent("meter-update", handler);
  },

  onRecordingStarted(handler: EventHandler<RecordingPayload>): Promise<UnlistenFn> {
    return listenToEvent("recording-started", handler);
  },

  onMeetingPromptUpdated(
    handler: EventHandler<MeetingPromptPayload | null>,
  ): Promise<UnlistenFn> {
    return listenToEvent("meeting-prompt-updated", handler);
  },

  onRecordingStopped(handler: EventHandler<ThreadDetail>): Promise<UnlistenFn> {
    return listenToEvent("recording-stopped", handler);
  },

  onTranscriptUpdate(handler: EventHandler<LiveTranscriptPayload>): Promise<UnlistenFn> {
    return listenToEvent("transcript-update", handler);
  },

  onFinalizationStatus(handler: EventHandler<FinalizationStatusPayload>): Promise<UnlistenFn> {
    return listenToEvent("finalization-status", handler);
  },

  onLegacyImportRequested(handler: () => void): Promise<UnlistenFn> {
    return listenToEvent("legacy-import-requested", handler);
  },
};
