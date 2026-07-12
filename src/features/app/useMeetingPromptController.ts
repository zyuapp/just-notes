import { useCallback } from "react";
import { api, getApiErrorMessage } from "../../api";
import { runRecordingStart } from "./recordingStart";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

export function useMeetingPromptController(
  dispatch: AppDispatch,
  refreshThreads: (nextSelectedId?: string) => Promise<void>,
) {
  const startMeetingRecording = useCallback(
    async (requestId: string) => {
      await runRecordingStart({
        dispatch,
        start: () => api.meetings.startRecording(requestId),
        refreshThreads,
      });
    },
    [dispatch, refreshThreads],
  );

  const dismissMeetingPrompt = useCallback(
    async (requestId: string) => {
      try {
        const meetingPrompt = await api.meetings.dismissPrompt(requestId);
        dispatch({ type: "meetingPromptChanged", meetingPrompt });
      } catch (error) {
        dispatch({ type: "failed", message: getApiErrorMessage(error) });
      }
    },
    [dispatch],
  );

  return { dismissMeetingPrompt, startMeetingRecording };
}
