import { useEffect } from "react";
import { api } from "../../api";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

const FINALIZATION_TERMINAL_STATES = new Set(["done", "failed", "cancelled"]);

export function useLiveTranscriptEvents(
  dispatch: AppDispatch,
  onFinalizationSettled: (threadId: string) => void,
) {
  useEffect(() => {
    const subscriptions = [
      api.events.onMeter((payload) => {
        dispatch({ type: "meterReceived", payload });
      }),
      api.events.onLiveTranscriptSegment((payload) => {
        dispatch({ type: "liveSegmentReceived", payload, updatedAtMs: Date.now() });
      }),
      api.events.onLiveTranscriptStatus((payload) => {
        dispatch({ type: "liveStatusReceived", payload });
      }),
      api.events.onLiveTranscriptError((payload) => {
        dispatch({ type: "liveErrorReceived", payload });
      }),
      api.events.onRecordingStopped((detail) => {
        dispatch({ type: "recordingStopped", detail });
      }),
      api.events.onFinalizationStatus((payload) => {
        dispatch({ type: "finalizationReceived", payload });
        if (FINALIZATION_TERMINAL_STATES.has(payload.state)) {
          onFinalizationSettled(payload.threadId);
        }
      }),
    ];

    return () => {
      for (const subscription of subscriptions) {
        subscription.then((dispose) => dispose()).catch(() => undefined);
      }
    };
  }, [dispatch, onFinalizationSettled]);
}
