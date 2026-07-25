import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import { recordingBlockers, type PrivacyPane } from "../lib/permissionStatus";
import { PermissionRow } from "./PermissionRow";
import { SettingsBanner } from "./SettingsBanner";

type SettingsPermissionsSectionProps = {
  permissions: PermissionsPayload | null;
  meetingAccess: MeetingAccessPayload | null;
  onOpenPrivacy: (pane: PrivacyPane) => void;
};

export function SettingsPermissionsSection({
  permissions,
  meetingAccess,
  onOpenPrivacy,
}: SettingsPermissionsSectionProps) {
  // The banner names whatever the rows below are already flagging, so it can
  // never advertise a problem the list does not show, or miss one it does.
  const blockers = recordingBlockers(permissions);
  const micBlocked = blockers.includes("Microphone");
  return (
    <section id="settings-permissions">
      {blockers.length > 0 && (
        <SettingsBanner
          tone={micBlocked ? "danger" : "warning"}
          title={micBlocked ? "Just Notes can't record right now" : "Recordings will be incomplete"}
          description={`${blockers.join(" and ")} ${
            blockers.length > 1 ? "are" : "is"
          } not granted. ${
            micBlocked
              ? "Starting a recording will fail."
              : "Only your microphone will be captured."
          }`}
        />
      )}

      <h3>Recording</h3>
      <PermissionRow
        label="Microphone"
        status={permissions?.microphone}
        grantedHint="Just Notes can capture your voice."
        blockedHint="Just Notes cannot record until this is enabled."
        pendingHint="macOS asks the first time you start a recording."
        pane="microphone"
        onOpenPrivacy={onOpenPrivacy}
      />
      <PermissionRow
        label="System audio"
        status={permissions?.systemAudio}
        grantedHint="Granted under Screen & System Audio Recording. Captures the other side of a call."
        blockedHint="Only your microphone is recorded, not the other side of the call. macOS asks the first time you record."
        pendingHint="macOS asks the first time you start a recording."
        pane="system-audio"
        onOpenPrivacy={onOpenPrivacy}
      />

      <h3 className="settings-group-heading">Meeting prompts</h3>
      <PermissionRow
        label="Calendar"
        status={meetingAccess?.calendarAuthorization}
        grantedHint="Read-only. Used for meeting titles and times."
        blockedHint="Meeting reminders cannot find your events."
        pendingHint="Connect your calendars from the Meetings section."
        pane="calendar"
        onOpenPrivacy={onOpenPrivacy}
      />
      <PermissionRow
        label="Notifications"
        status={meetingAccess?.notificationAuthorization}
        grantedHint="Meeting prompts can appear."
        blockedHint="Meeting prompts cannot appear."
        pendingHint="macOS asks when you turn on meeting reminders."
        pane="notifications"
        onOpenPrivacy={onOpenPrivacy}
      />

      <p className="settings-hint settings-note">
        Status re-checks whenever you switch back to Just Notes.
      </p>
    </section>
  );
}
