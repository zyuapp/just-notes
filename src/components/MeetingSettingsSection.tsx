import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { MeetingCalendarConnect } from "./MeetingCalendarConnect";
import { MeetingCallDetectionControls } from "./MeetingCallDetectionControls";
import { MeetingSettingsConnected } from "./MeetingSettingsConnected";
import type { MeetingSettingsActions } from "./settingsViewTypes";
import type { PrivacyPane } from "../lib/permissionStatus";

type MeetingSettingsSectionProps = MeetingSettingsActions & {
  settings: AppSettings;
  access: MeetingAccessPayload | null;
  onOpenPrivacy: (pane: PrivacyPane) => void;
};

export function MeetingSettingsSection(props: MeetingSettingsSectionProps) {
  const { access, requestingAccess, onOpenPrivacy, onRequestCalendarAccess } = props;
  const calendarAuthorized = access?.calendarAuthorization === "authorized";

  return (
    <section className="meeting-settings">
      {calendarAuthorized && access ? (
        <MeetingSettingsConnected {...props} access={access} />
      ) : (
        <MeetingCalendarConnect
          denied={access?.calendarAuthorization === "denied"}
          busy={requestingAccess}
          onConnect={onRequestCalendarAccess}
          onOpenSettings={() => onOpenPrivacy("calendar")}
        />
      )}
      <MeetingCallDetectionControls
        settings={props.settings}
        access={access}
        requestingAccess={requestingAccess}
        onToggleCallDetection={props.onToggleCallDetection}
        onResetIgnoredCallApps={props.onResetIgnoredCallApps}
        onRequestNotificationAccess={props.onRequestNotificationAccess}
        onOpenPrivacy={onOpenPrivacy}
      />
    </section>
  );
}
