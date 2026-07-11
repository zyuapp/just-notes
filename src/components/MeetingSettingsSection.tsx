import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { permissionLabel, SettingsToggle } from "./SettingsControls";

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

const REMINDER_MINUTES = [1, 5, 10];

export function MeetingSettingsSection({
  settings,
  access,
  onRequestAccess,
  onToggleCalendar,
  onToggleReminders,
  onSetReminderMinutes,
  onToggleEndReminders,
  onOpenPrivacy,
  busy,
}: MeetingSettingsSectionProps) {
  const calendarAuthorized = access?.calendarAuthorization === "authorized";
  const notificationAuthorized = access?.notificationAuthorization === "authorized";
  const availableIds = new Set(access?.calendars.map((calendar) => calendar.id) ?? []);
  const hasSelectedCalendar = settings.meetingCalendarIds.some((id) => availableIds.has(id));

  return (
    <section>
      <h3>Meeting reminders</h3>
      {!calendarAuthorized ? (
        <div className="calendar-connect">
          <p className="settings-hint">
            Just Notes reads meeting titles and times to ask when recording should start and stop.
            It never creates, edits, or deletes calendar events.
          </p>
          <p className="settings-hint calendar-access-note">
            macOS calls this “Full Access” because it does not offer read-only access to apps that
            need to inspect existing events.
          </p>
          <div className="settings-row-actions">
            {access?.calendarAuthorization === "denied" ? (
              <button type="button" onClick={() => onOpenPrivacy("calendar")} disabled={busy}>
                Open System Settings
              </button>
            ) : (
              <button type="button" onClick={onRequestAccess} disabled={busy}>
                Connect calendars
              </button>
            )}
          </div>
        </div>
      ) : (
        <>
          <div className="settings-row">
            <div>
              <strong>Calendar access</strong>
              <p className="settings-hint">{permissionLabel(access.calendarAuthorization)}</p>
            </div>
          </div>
          <fieldset className="calendar-list">
            <legend>Calendars</legend>
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
          </fieldset>
          <SettingsToggle
            label="Recording reminders"
            description={
              hasSelectedCalendar
                ? "Ask before selected-calendar meetings begin."
                : "Select at least one calendar to enable reminders."
            }
            checked={settings.meetingRemindersEnabled && hasSelectedCalendar}
            onToggle={onToggleReminders}
            disabled={!hasSelectedCalendar || busy}
          />
          <div className="settings-row">
            <div>
              <strong>Ask before meetings</strong>
              <p className="settings-hint">Choose how early the start prompt appears.</p>
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
            description="Ask at the scheduled end; Keep recording asks again in 10 minutes."
            checked={settings.meetingEndReminders}
            onToggle={onToggleEndReminders}
            disabled={busy}
          />
          {!notificationAuthorized && (
            <div className="settings-row">
              <div>
                <strong>Notifications need access</strong>
                <p className="settings-hint">Allow notifications so meeting prompts can appear.</p>
              </div>
              <div className="settings-row-actions">
                {access?.notificationAuthorization === "denied" ? (
                  <button type="button" onClick={() => onOpenPrivacy("notifications")} disabled={busy}>
                    Open System Settings
                  </button>
                ) : (
                  <button type="button" onClick={onRequestAccess} disabled={busy}>
                    Allow notifications
                  </button>
                )}
              </div>
            </div>
          )}
        </>
      )}
    </section>
  );
}
