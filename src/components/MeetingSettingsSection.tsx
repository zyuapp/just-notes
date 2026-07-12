import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { MeetingCalendarConnect } from "./MeetingCalendarConnect";
import { MeetingSettingsConnected } from "./MeetingSettingsConnected";

type MeetingSettingsSectionProps = {
  settings: AppSettings;
  access: MeetingAccessPayload | null;
  onRequestAccess: () => void;
  onToggleCalendar: (calendarId: string) => void;
  onToggleReminders: () => void;
  onSetReminderMinutes: (minutes: number) => void;
  onToggleEndReminders: () => void;
  onOpenPrivacy: (pane: "calendar" | "notifications") => void;
  busy: boolean;
};

export function MeetingSettingsSection(props: MeetingSettingsSectionProps) {
  const { access, busy, onOpenPrivacy, onRequestAccess } = props;
  const calendarAuthorized = access?.calendarAuthorization === "authorized";

  return (
    <section className="meeting-settings">
      {calendarAuthorized && access ? (
        <MeetingSettingsConnected {...props} access={access} />
      ) : (
        <MeetingCalendarConnect
          denied={access?.calendarAuthorization === "denied"}
          busy={busy}
          onConnect={onRequestAccess}
          onOpenSettings={() => onOpenPrivacy("calendar")}
        />
      )}
    </section>
  );
}
