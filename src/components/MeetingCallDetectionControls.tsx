import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import type { PrivacyPane } from "../lib/permissionStatus";
import { Button } from "./Button";
import { SettingsToggle } from "./SettingsControls";
import { SettingsRow } from "./SettingsRow";

type MeetingCallDetectionControlsProps = {
  settings: AppSettings;
  access: MeetingAccessPayload | null;
  requestingAccess: boolean;
  onToggleCallDetection: () => void;
  onResetIgnoredCallApps: () => void;
  onRequestNotificationAccess: () => void;
  onOpenPrivacy: (pane: PrivacyPane) => void;
};

export function MeetingCallDetectionControls({
  settings,
  access,
  requestingAccess,
  onToggleCallDetection,
  onResetIgnoredCallApps,
  onRequestNotificationAccess,
  onOpenPrivacy,
}: MeetingCallDetectionControlsProps) {
  const notifications = access?.notificationAuthorization;
  const notificationsAllowed = notifications === "authorized";
  const ignored = settings.callDetectionIgnoredApps;

  return (
    <section className="meeting-settings-group">
      <h3>Call detection</h3>
      <SettingsToggle
        label="Offer to record calls"
        description="When Zoom, Teams, Slack, FaceTime, Webex, Discord, or a browser call uses the microphone. Calls during a watched calendar meeting are named after it and follow its recording settings."
        checked={settings.callDetectionEnabled}
        onToggle={onToggleCallDetection}
      />
      {settings.callDetectionEnabled && access && !notificationsAllowed && (
        <SettingsRow
          label="Notifications are off"
          description="Call offers appear as notifications."
          tone="warning"
        >
          <Button
            size="compact"
            variant="primary"
            disabled={requestingAccess}
            onClick={
              notifications === "denied"
                ? () => onOpenPrivacy("notifications")
                : onRequestNotificationAccess
            }
          >
            {notifications === "denied" ? "Open System Settings" : "Allow notifications"}
          </Button>
        </SettingsRow>
      )}
      {ignored.length > 0 && (
        <SettingsRow label="Never offered" description={ignored.join(", ")}>
          <Button size="compact" onClick={onResetIgnoredCallApps}>
            Reset
          </Button>
        </SettingsRow>
      )}
    </section>
  );
}
