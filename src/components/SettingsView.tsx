import { useState } from "react";
import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { MeetingSettingsSection } from "./MeetingSettingsSection";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { SettingsToggle } from "./SettingsControls";
import { SettingsNavigation, type SettingsSectionId } from "./SettingsNavigation";
import { SettingsPermissionsSection } from "./SettingsPermissionsSection";
import { TranscriptionSettingsSection } from "./TranscriptionSettingsSection";
import { useDismissOnEscape } from "./useDismissOnEscape";

const SECTION_COPY: Record<SettingsSectionId, { title: string; description: string }> = {
  storage: { title: "Storage", description: "Choose where recordings live and which files are kept." },
  transcription: { title: "Transcription", description: "Process every recording locally on this Mac." },
  meetings: { title: "Meetings", description: "Get a prompt when a calendar meeting is about to begin or end." },
  permissions: { title: "Permissions", description: "Review the system access Just Notes uses." },
};

type SettingsViewProps = {
  settings: AppSettings;
  appInfo: AppInfo | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  permissions: PermissionsPayload | null;
  meetingAccess: MeetingAccessPayload | null;
  onClose: () => void;
  onRevealFolder: () => void;
  onToggleRawAudio: () => void;
  onToggleMarkdownCopy: () => void;
  onRequestMeetingAccess: () => void;
  onToggleMeetingCalendar: (calendarId: string) => void;
  onToggleMeetingReminders: () => void;
  onSetMeetingReminderMinutes: (minutes: number) => void;
  onToggleMeetingEndReminders: () => void;
  meetingSettingsBusy: boolean;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
  onDeleteModel: () => void;
  onOpenPrivacy: (pane: "microphone" | "system-audio" | "calendar" | "notifications") => void;
};

export function SettingsView({
  settings,
  appInfo,
  transcriptionStatus,
  permissions,
  meetingAccess,
  onClose,
  onRevealFolder,
  onToggleRawAudio,
  onToggleMarkdownCopy,
  onRequestMeetingAccess,
  onToggleMeetingCalendar,
  onToggleMeetingReminders,
  onSetMeetingReminderMinutes,
  onToggleMeetingEndReminders,
  meetingSettingsBusy,
  onStartModelDownload,
  onCancelModelDownload,
  onDeleteModel,
  onOpenPrivacy,
}: SettingsViewProps) {
  const [activeSection, setActiveSection] = useState<SettingsSectionId>("storage");
  const activeSectionCopy = SECTION_COPY[activeSection];
  useDismissOnEscape(onClose);

  return (
    <div className="settings-overlay" role="dialog" aria-modal="true" aria-label="Settings">
      <SettingsNavigation activeSection={activeSection} onSelect={setActiveSection} onClose={onClose} />

      <div className="settings-content">
        <div className="settings-panel">
          <header className="settings-titlebar">
            <h2>{activeSectionCopy.title}</h2>
            <p>{activeSectionCopy.description}</p>
          </header>

        {activeSection === "storage" && <section id="settings-storage">
          <div className="settings-row">
            <div>
              <strong>Transcripts folder</strong>
              <p className="settings-path">{appInfo?.threadsDir ?? "…"}</p>
            </div>
            <div className="settings-row-actions">
              <button type="button" onClick={onRevealFolder}>Reveal</button>
            </div>
          </div>
          <SettingsToggle
            label="Save raw audio"
            description="Keep mic.wav and system.wav after the transcript is polished."
            checked={settings.saveRawAudio}
            onToggle={onToggleRawAudio}
          />
          <SettingsToggle
            label="Create Markdown copies"
            description="Write a transcript.md beside every recording when it finishes."
            checked={settings.markdownCopy}
            onToggle={onToggleMarkdownCopy}
          />
        </section>}

        {activeSection === "transcription" && <div id="settings-transcription">
          <TranscriptionSettingsSection
            transcriptionStatus={transcriptionStatus}
            onStartModelDownload={onStartModelDownload}
            onCancelModelDownload={onCancelModelDownload}
            onDeleteModel={onDeleteModel}
          />
        </div>}

        {activeSection === "meetings" && <div id="settings-meetings">
          <MeetingSettingsSection
            settings={settings}
            access={meetingAccess}
            onRequestAccess={onRequestMeetingAccess}
            onToggleCalendar={onToggleMeetingCalendar}
            onToggleReminders={onToggleMeetingReminders}
            onSetReminderMinutes={onSetMeetingReminderMinutes}
            onToggleEndReminders={onToggleMeetingEndReminders}
            onOpenPrivacy={onOpenPrivacy}
            busy={meetingSettingsBusy}
          />
        </div>}

          {activeSection === "permissions" && (
            <SettingsPermissionsSection permissions={permissions} onOpenPrivacy={onOpenPrivacy} />
          )}
        </div>
      </div>
    </div>
  );
}
