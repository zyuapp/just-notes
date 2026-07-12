import { CalendarDays, ShieldCheck } from "lucide-react";

type MeetingCalendarConnectProps = {
  denied: boolean;
  busy: boolean;
  onConnect: () => void;
  onOpenSettings: () => void;
};

export function MeetingCalendarConnect({
  denied,
  busy,
  onConnect,
  onOpenSettings,
}: MeetingCalendarConnectProps) {
  return (
    <div className="calendar-connect">
      <div className="calendar-connect-main">
        <span className="calendar-connect-icon" aria-hidden="true">
          <CalendarDays size={20} />
        </span>
        <div className="calendar-connect-copy">
          <h3>Connect your calendars</h3>
          <p>
            Just Notes reads meeting titles and times so it can ask when recording should start and
            stop.
          </p>
          <button
            type="button"
            className="calendar-connect-button"
            onClick={denied ? onOpenSettings : onConnect}
            disabled={busy}
          >
            {denied ? "Open System Settings" : "Connect calendars"}
          </button>
        </div>
      </div>
      <div className="calendar-privacy-note">
        <ShieldCheck size={17} aria-hidden="true" />
        <div>
          <strong>Why macOS asks for Full Access</strong>
          <p>
            macOS does not offer read-only calendar permission. Just Notes never creates, changes,
            or deletes calendar events.
          </p>
        </div>
      </div>
    </div>
  );
}
