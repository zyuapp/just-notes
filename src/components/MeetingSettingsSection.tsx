import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { MeetingCalendarConnect } from "./MeetingCalendarConnect";
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
    </section>
  );
}
