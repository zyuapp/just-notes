import { useEffect } from "react";
import { api } from "../../api";
import { createMeetingPromptHydration } from "./meetingPromptHydration";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

const FINALIZATION_TERMINAL_STATES = new Set(["done", "failed", "cancelled"]);

export function useAppEvents(
  dispatch: AppDispatch,
  onFinalizationSettled: (threadId: string) => void,
  onRecordingStarted: (threadId: string) => void,
) {
  useEffect(() => {
    const promptHydration = createMeetingPromptHydration((meetingPrompt) => {
      dispatch({ type: "meetingPromptChanged", meetingPrompt });
    });
    const meetingPromptSubscription = api.events.onMeetingPromptUpdated(promptHydration.receive);
    meetingPromptSubscription
      .then(() => promptHydration.hydrate(api.meetings.getPrompt))
      .catch(() => undefined);

    const subscriptions = [
      meetingPromptSubscription,
      api.events.onMeter((payload) => {
        dispatch({ type: "meterReceived", payload });
      }),
      api.events.onRecordingStarted((payload) => {
        dispatch({ type: "recordingStarted", payload });
        onRecordingStarted(payload.thread.summary.id);
      }),
      api.events.onRecordingStopped((detail) => {
        dispatch({ type: "recordingStopped", detail });
      }),
      api.events.onTranscriptUpdate((payload) => {
        dispatch({ type: "liveSegmentReceived", payload });
      }),
      api.events.onFinalizationStatus((payload) => {
        dispatch({ type: "finalizationReceived", payload });
        if (FINALIZATION_TERMINAL_STATES.has(payload.state)) {
          onFinalizationSettled(payload.threadId);
        }
      }),
    ];

    return () => {
      promptHydration.dispose();
      for (const subscription of subscriptions) {
        subscription.then((dispose) => dispose()).catch(() => undefined);
      }
    };
  }, [dispatch, onFinalizationSettled, onRecordingStarted]);
}
