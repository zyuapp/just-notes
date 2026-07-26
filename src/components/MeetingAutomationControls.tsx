import type { AppSettings } from "../bindings/AppSettings";
import { SettingsToggle } from "./SettingsControls";

type MeetingAutomationControlsProps = {
  settings: AppSettings;
  canAutomate: boolean;
  blockedReason: string;
  onToggleAutoRecord: () => void;
  onToggleAutoStop: () => void;
  onToggleRequireAttendees: () => void;
};

export function MeetingAutomationControls({
  settings,
  canAutomate,
  blockedReason,
  onToggleAutoRecord,
  onToggleAutoStop,
  onToggleRequireAttendees,
}: MeetingAutomationControlsProps) {
  const automationEnabled = canAutomate && settings.meetingAutoRecordEnabled;

  return (
    <section className="meeting-settings-group">
      <h3>Automation</h3>
      <SettingsToggle
        label="Automatically record meetings"
        description={
          canAutomate
            ? "Start recording at the scheduled meeting time."
            : blockedReason
        }
        checked={automationEnabled}
        onToggle={onToggleAutoRecord}
        disabled={!canAutomate}
      />
      <SettingsToggle
        label="Only meetings with other attendees"
        description="Ignore focus blocks and personal calendar events."
        checked={settings.meetingAutoRecordRequiresAttendees}
        onToggle={onToggleRequireAttendees}
        disabled={!automationEnabled}
      />
      <SettingsToggle
        label="Stop at the scheduled end"
        description="Warn one minute before stopping so you can keep recording."
        checked={settings.meetingAutoStopEnabled}
        onToggle={onToggleAutoStop}
        disabled={!automationEnabled}
      />
    </section>
  );
}
