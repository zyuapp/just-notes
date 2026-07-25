import { getApiErrorMessage } from "../../api";
import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

type MeetingAccessRequestOptions = {
  dispatch: AppDispatch;
  request: () => Promise<MeetingAccessPayload>;
  refresh: () => Promise<MeetingAccessPayload>;
};

export async function runMeetingAccessRequest({
  dispatch,
  request,
  refresh,
}: MeetingAccessRequestOptions) {
  dispatch({ type: "errorCleared" });
  try {
    const meetingAccess = await request();
    dispatch({ type: "meetingAccessLoaded", meetingAccess });
    return;
  } catch (error) {
    try {
      const meetingAccess = await refresh();
      dispatch({ type: "meetingAccessLoaded", meetingAccess });
    } catch {
      // Preserve the original permission-request error.
    }
    dispatch({ type: "failed", message: getApiErrorMessage(error) });
  }
}
