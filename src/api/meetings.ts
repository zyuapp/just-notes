import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { invokeCommand } from "./transport";

export const meetingsApi = {
  getAccessStatus(): Promise<MeetingAccessPayload> {
    return invokeCommand("get_meeting_access_status");
  },

  requestAccess(): Promise<MeetingAccessPayload> {
    return invokeCommand("request_meeting_access");
  },
};
