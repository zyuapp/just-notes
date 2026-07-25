import type { MeetingCalendarPayload } from "../bindings/MeetingCalendarPayload";

type CalendarOptionProps = {
  calendar: MeetingCalendarPayload;
  checked: boolean;
  pending: boolean;
  onToggle: (calendarId: string) => void;
};

export function CalendarOption({ calendar, checked, pending, onToggle }: CalendarOptionProps) {
  return (
    <label className="calendar-option">
      <span className="settings-check">
        <input
          type="checkbox"
          checked={checked}
          onChange={() => onToggle(calendar.id)}
        />
        <span className="settings-check-box" aria-hidden="true" />
      </span>
      <span className="calendar-swatch" aria-hidden="true" style={{ background: calendar.color }} />
      <span className="calendar-option-title">{calendar.title}</span>
      {pending && <span className="calendar-option-pending">Saving…</span>}
    </label>
  );
}
