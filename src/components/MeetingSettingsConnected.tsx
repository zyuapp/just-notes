import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { hasWatchedCalendar } from "../lib/meetingCalendars";
import { meetingPromptsBlocked } from "../lib/permissionStatus";
import { Button } from "./Button";
import { CalendarPicker } from "./CalendarPicker";
import { MeetingAutomationControls } from "./MeetingAutomationControls";
import { MeetingReminderControls } from "./MeetingReminderControls";
import { SettingsBanner } from "./SettingsBanner";
import { SettingsRow } from "./SettingsRow";
import { StatusPill } from "./StatusPill";
import type { MeetingSettingsActions } from "./settingsViewTypes";
import type { PrivacyPane } from "../lib/permissionStatus";

type MeetingSettingsConnectedProps = MeetingSettingsActions & {
  settings: AppSettings;
  access: MeetingAccessPayload;
  onOpenPrivacy: (pane: PrivacyPane) => void;
};

export function MeetingSettingsConnected(props: MeetingSettingsConnectedProps) {
  const { access, settings } = props;
  const hasSelectedCalendar = hasWatchedCalendar(settings.meetingCalendarIds, access.calendars);
  const notificationsBlocked = meetingPromptsBlocked(access);
  const canRemind = hasSelectedCalendar && !notificationsBlocked;

  return (
    <>
      {notificationsBlocked && (
        <SettingsBanner
          tone="warning"
          title="Meeting prompts can't appear"
          description="Notifications are blocked for Just Notes, so nothing below will fire until you allow them."
        >
          <Button
            size="compact"
            variant="primary"
            disabled={props.requestingAccess}
            onClick={
              access.notificationAuthorization === "denied"
                ? () => props.onOpenPrivacy("notifications")
                : props.onRequestNotificationAccess
            }
          >
            {access.notificationAuthorization === "denied"
              ? "Open System Settings"
              : "Allow notifications"}
          </Button>
        </SettingsBanner>
      )}

      <section className="meeting-settings-group">
        <SettingsRow label="Calendar access" description="Read-only access to macOS Calendar.">
          <StatusPill tone="ok" label="Connected" />
        </SettingsRow>
      </section>

      <section className="meeting-settings-group">
        <h3>Calendars to watch</h3>
        <CalendarPicker
          calendars={access.calendars}
          selectedIds={settings.meetingCalendarIds}
          pendingIds={props.pendingCalendarIds}
          onToggle={props.onToggleCalendar}
        />
      </section>

      <MeetingReminderControls
        settings={settings}
        canRemind={canRemind}
        blockedReason={
          notificationsBlocked
            ? "Blocked by notification permission."
            : "Watch at least one calendar to enable reminders."
        }
        onToggleReminders={props.onToggleReminders}
        onSetReminderMinutes={props.onSetReminderMinutes}
        onToggleEndReminders={props.onToggleEndReminders}
      />

      <MeetingAutomationControls
        settings={settings}
        canAutomate={canRemind && settings.meetingRemindersEnabled}
        blockedReason={
          !settings.meetingRemindersEnabled
            ? "Turn on recording reminders first."
            : notificationsBlocked
              ? "Blocked by notification permission."
              : "Watch at least one calendar to enable automation."
        }
        onToggleAutoRecord={props.onToggleAutoRecord}
        onToggleAutoStop={props.onToggleAutoStop}
        onToggleRequireAttendees={props.onToggleRequireAttendees}
      />

      {!hasSelectedCalendar && settings.meetingRemindersEnabled && (
        <p className="settings-callout">
          Reminders stay switched on and resume as soon as you watch a calendar again.
        </p>
      )}
    </>
  );
}
