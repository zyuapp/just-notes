import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { SettingsToggle } from "./SettingsControls";

type MeetingSettingsConnectedProps = {
  settings: AppSettings;
  access: MeetingAccessPayload;
  onRequestAccess: () => void;
  onToggleCalendar: (calendarId: string) => void;
  onToggleReminders: () => void;
  onSetReminderMinutes: (minutes: number) => void;
  onToggleEndReminders: () => void;
  onOpenPrivacy: (pane: "calendar" | "notifications") => void;
  busy: boolean;
};

const REMINDER_MINUTES = [1, 5, 10];

export function MeetingSettingsConnected({
  settings, access, onRequestAccess, onToggleCalendar, onToggleReminders,
  onSetReminderMinutes, onToggleEndReminders, onOpenPrivacy, busy,
}: MeetingSettingsConnectedProps) {
  const availableIds = new Set(access.calendars.map((calendar) => calendar.id));
  const hasSelectedCalendar = settings.meetingCalendarIds.some((id) => availableIds.has(id));

  return <>
    <section className="meeting-settings-group">
      <h3>Calendar access</h3>
      <div className="settings-row">
        <div>
          <strong>Calendar access</strong>
          <p className="settings-hint">Connected to macOS Calendar</p>
        </div>
        <span className="calendar-connected-status"><i aria-hidden="true" />Connected</span>
      </div>
    </section>

    <section className="meeting-settings-group">
      <h3>Calendars</h3>
      <div className="calendar-list">
        {access.calendars.map((calendar) => (
          <label key={calendar.id} className="calendar-option">
            <input
              type="checkbox"
              checked={settings.meetingCalendarIds.includes(calendar.id)}
              onChange={() => onToggleCalendar(calendar.id)}
              disabled={busy}
            />
            <span>{calendar.title}</span>
          </label>
        ))}
        {access.calendars.length === 0 && (
          <p className="settings-hint">No event calendars are available in macOS Calendar.</p>
        )}
      </div>
    </section>

    <section className="meeting-settings-group">
      <h3>Reminders</h3>
      <SettingsToggle
        label="Recording reminders"
        description={hasSelectedCalendar
          ? "Ask before meetings on selected calendars."
          : "Select at least one calendar to enable reminders."}
        checked={settings.meetingRemindersEnabled && hasSelectedCalendar}
        onToggle={onToggleReminders}
        disabled={!hasSelectedCalendar || busy}
      />
      <div className="settings-row">
        <div>
          <strong>Ask before meetings</strong>
          <p className="settings-hint">Choose when the start prompt appears.</p>
        </div>
        <div className="settings-segmented" aria-label="Meeting reminder lead time">
          {REMINDER_MINUTES.map((minutes) => (
            <button
              key={minutes}
              type="button"
              className={settings.meetingReminderMinutes === minutes ? "selected" : ""}
              onClick={() => onSetReminderMinutes(minutes)}
              aria-pressed={settings.meetingReminderMinutes === minutes}
              disabled={busy}
            >
              {minutes}m
            </button>
          ))}
        </div>
      </div>
      <SettingsToggle
        label="End reminders"
        description="Ask at the scheduled end, then again in 10 minutes."
        checked={settings.meetingEndReminders}
        onToggle={onToggleEndReminders}
        disabled={busy}
      />
    </section>

    {access.notificationAuthorization !== "authorized" && (
      <section className="meeting-settings-group">
        <h3>Notifications</h3>
        <div className="settings-row">
          <div>
            <strong>Notifications need access</strong>
            <p className="settings-hint">Allow notifications so meeting prompts can appear.</p>
          </div>
          <div className="settings-row-actions">
            <button
              type="button"
              disabled={busy}
              onClick={access.notificationAuthorization === "denied"
                ? () => onOpenPrivacy("notifications")
                : onRequestAccess}
            >
              {access.notificationAuthorization === "denied"
                ? "Open System Settings"
                : "Allow notifications"}
            </button>
          </div>
        </div>
      </section>
    )}
  </>;
}
