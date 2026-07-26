import type { AppSettings } from "../bindings/AppSettings";
import { SettingsToggle } from "./SettingsControls";

type MeetingAutomationControlsProps = {
  settings: AppSettings;
  canAutomate: boolean;
  blockedReason: string;
  onToggleAutoRecord: () => void;
};

export function MeetingAutomationControls({
  settings,
  canAutomate,
  blockedReason,
  onToggleAutoRecord,
}: MeetingAutomationControlsProps) {
  return (
    <section className="meeting-settings-group">
      <h3>Automation</h3>
      <SettingsToggle
        label="Automatically record meetings"
        description={
          canAutomate
            ? "Starts recording meetings with other attendees, then stops at the scheduled end. You can skip or keep recording."
            : blockedReason
        }
        checked={canAutomate && settings.meetingAutoRecordEnabled}
        onToggle={onToggleAutoRecord}
        disabled={!canAutomate}
      />
    </section>
  );
}
