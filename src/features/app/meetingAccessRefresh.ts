import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";

export function createMeetingAccessRefresh(
  onAccess: (meetingAccess: MeetingAccessPayload) => void,
) {
  let permissionRequestInFlight = false;
  let revision = 0;

  return {
    beginPermissionRequest() {
      permissionRequestInFlight = true;
      revision += 1;
    },
    endPermissionRequest() {
      permissionRequestInFlight = false;
    },
    invalidate() {
      revision += 1;
    },
    async refresh(load: () => Promise<MeetingAccessPayload>) {
      if (permissionRequestInFlight) return;
      const startingRevision = ++revision;
      const meetingAccess = await load();
      if (!permissionRequestInFlight && startingRevision === revision) {
        onAccess(meetingAccess);
      }
    },
  };
}
