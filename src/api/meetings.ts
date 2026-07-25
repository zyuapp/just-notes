import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import type { MeetingPromptPayload } from "../bindings/MeetingPromptPayload";
import type { RecordingPayload } from "../bindings/RecordingPayload";
import { invokeCommand } from "./transport";

export const meetingsApi = {
  getAccessStatus(): Promise<MeetingAccessPayload> {
    return invokeCommand("get_meeting_access_status");
  },

  requestCalendarAccess(): Promise<MeetingAccessPayload> {
    return invokeCommand("request_meeting_calendar_access");
  },

  requestNotificationAccess(): Promise<MeetingAccessPayload> {
    return invokeCommand("request_meeting_notification_access");
  },

  getPrompt(): Promise<MeetingPromptPayload | null> {
    return invokeCommand("get_meeting_prompt");
  },

  startRecording(requestId: string): Promise<RecordingPayload> {
    return invokeCommand("start_meeting_recording", { requestId });
  },

  dismissPrompt(requestId: string): Promise<MeetingPromptPayload | null> {
    return invokeCommand("dismiss_meeting_prompt", { requestId });
  },
};
