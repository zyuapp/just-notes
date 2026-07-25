import type { AppSettings } from "../bindings/AppSettings";
import { SettingsRow } from "./SettingsRow";
import { SettingsToggle } from "./SettingsControls";

const REMINDER_MINUTES = [1, 5, 10];

type MeetingReminderControlsProps = {
  settings: AppSettings;
  // False when no calendar is watched or notifications are blocked: every
  // control below is gated on the same condition so none of them can be armed
  // for a prompt that cannot fire.
  canRemind: boolean;
  blockedReason: string;
  onToggleReminders: () => void;
  onSetReminderMinutes: (minutes: number) => void;
  onToggleEndReminders: () => void;
};

export function MeetingReminderControls({
  settings,
  canRemind,
  blockedReason,
  onToggleReminders,
  onSetReminderMinutes,
  onToggleEndReminders,
}: MeetingReminderControlsProps) {
  return (
    <section className="meeting-settings-group">
      <h3>Reminders</h3>
      <SettingsToggle
        label="Recording reminders"
        description={canRemind ? "Ask before meetings on the calendars you watch." : blockedReason}
        checked={settings.meetingRemindersEnabled && canRemind}
        onToggle={onToggleReminders}
        disabled={!canRemind}
      />
      <SettingsRow label="Ask before meetings" description="Choose when the start prompt appears.">
        <div
          className="settings-segmented"
          role="radiogroup"
          aria-label="Meeting reminder lead time"
        >
          {REMINDER_MINUTES.map((minutes) => (
            <button
              key={minutes}
              type="button"
              role="radio"
              className={settings.meetingReminderMinutes === minutes ? "selected" : ""}
              onClick={() => onSetReminderMinutes(minutes)}
              aria-checked={settings.meetingReminderMinutes === minutes}
              disabled={!canRemind}
            >
              {minutes}m
            </button>
          ))}
        </div>
      </SettingsRow>
      <SettingsToggle
        label="End reminders"
        description="Ask at the scheduled end, then again in 10 minutes."
        checked={settings.meetingEndReminders && canRemind}
        onToggle={onToggleEndReminders}
        disabled={!canRemind}
      />
    </section>
  );
}
