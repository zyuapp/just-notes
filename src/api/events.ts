import { listen } from "@tauri-apps/api/event";
import type { LiveTranscriptSegmentPayload } from "../bindings/LiveTranscriptSegmentPayload";
import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
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

  onLiveTranscriptSegment(
    handler: EventHandler<LiveTranscriptSegmentPayload>,
  ): Promise<UnlistenFn> {
    return listenToEvent("live-transcript-segment", handler);
  },

  onLiveTranscriptStatus(handler: EventHandler<LiveTranscriptStatusPayload>): Promise<UnlistenFn> {
    return listenToEvent("live-transcript-status", handler);
  },

  onLiveTranscriptError(handler: EventHandler<LiveTranscriptStatusPayload>): Promise<UnlistenFn> {
    return listenToEvent("live-transcript-error", handler);
  },
};
