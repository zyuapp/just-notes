import { listen } from "@tauri-apps/api/event";
import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { MeterPayload } from "../bindings/MeterPayload";
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

  onIndicatorState(handler: EventHandler<boolean>): Promise<UnlistenFn> {
    return listenToEvent("indicator-state", handler);
  },

  onIndicatorHover(handler: EventHandler<boolean>): Promise<UnlistenFn> {
    return listenToEvent("indicator-hover", handler);
  },

  onRecordingStopped(handler: EventHandler<ThreadDetail>): Promise<UnlistenFn> {
    return listenToEvent("recording-stopped", handler);
  },

  onFinalizationStatus(handler: EventHandler<FinalizationStatusPayload>): Promise<UnlistenFn> {
    return listenToEvent("finalization-status", handler);
  },
};
