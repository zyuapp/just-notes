import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";

export function createMeetingAccessRefresh(
  onAccess: (meetingAccess: MeetingAccessPayload) => void,
) {
  let permissionRequestInFlight = false;
  let queuedLoad: (() => Promise<MeetingAccessPayload>) | null = null;
  let revision = 0;

  const refresh = async (load: () => Promise<MeetingAccessPayload>) => {
    if (permissionRequestInFlight) {
      queuedLoad = load;
      return;
    }
    const startingRevision = ++revision;
    const meetingAccess = await load();
    if (startingRevision === revision) onAccess(meetingAccess);
  };

  return {
    beginPermissionRequest() {
      if (permissionRequestInFlight) return false;
      permissionRequestInFlight = true;
      revision += 1;
      return true;
    },
    async endPermissionRequest() {
      permissionRequestInFlight = false;
      const load = queuedLoad;
      queuedLoad = null;
      if (load) await refresh(load);
    },
    invalidate() {
      queuedLoad = null;
      revision += 1;
    },
    refresh,
  };
}
