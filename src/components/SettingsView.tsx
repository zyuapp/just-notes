import { useState } from "react";
import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import { MeetingSettingsSection } from "./MeetingSettingsSection";
import { PrivacyLegalSection } from "./PrivacyLegalSection";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { SettingsNavigation, type SettingsSectionId } from "./SettingsNavigation";
import { SettingsPermissionsSection } from "./SettingsPermissionsSection";
import { StorageSettingsSection } from "./StorageSettingsSection";
import { TranscriptionSettingsSection } from "./TranscriptionSettingsSection";
import { useDismissOnEscape } from "./useDismissOnEscape";

const SECTION_COPY: Record<SettingsSectionId, { title: string; description: string }> = {
  storage: { title: "Storage", description: "Choose where recordings live and which files are kept." },
  transcription: { title: "Transcription", description: "Process every recording locally on this Mac." },
  meetings: { title: "Meetings", description: "Get a prompt when a calendar meeting is about to begin or end." },
  permissions: { title: "Permissions", description: "Review the system access Just Notes uses." },
  privacy: { title: "Privacy & Legal", description: "See how local data and third-party software are handled." },
};

type SettingsViewProps = {
  settings: AppSettings;
  appInfo: AppInfo | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  permissions: PermissionsPayload | null;
  meetingAccess: MeetingAccessPayload | null;
  onClose: () => void;
  onRevealFolder: () => void;
  onChooseTranscriptsFolder: () => void;
  onImportLegacyData: () => void;
  onUseDefaultTranscriptsFolder: () => void;
  onToggleRawAudio: () => void;
  onToggleMarkdownCopy: () => void;
  onRequestMeetingAccess: () => void;
  onToggleMeetingCalendar: (calendarId: string) => void;
  onToggleMeetingReminders: () => void;
  onSetMeetingReminderMinutes: (minutes: number) => void;
  onToggleMeetingEndReminders: () => void;
  meetingSettingsBusy: boolean;
  storageBusy: boolean;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
  onDeleteModel: () => void;
  onOpenPrivacy: (pane: "microphone" | "system-audio" | "calendar" | "notifications") => void;
  onOpenExternalUrl: (url: string) => void;
  onOpenLegalDocument: (document: "privacy" | "notices") => void;
};

export function SettingsView({
  settings,
  appInfo,
  transcriptionStatus,
  permissions,
  meetingAccess,
  onClose,
  onRevealFolder,
  onChooseTranscriptsFolder,
  onImportLegacyData,
  onUseDefaultTranscriptsFolder,
  onToggleRawAudio,
  onToggleMarkdownCopy,
  onRequestMeetingAccess,
  onToggleMeetingCalendar,
  onToggleMeetingReminders,
  onSetMeetingReminderMinutes,
  onToggleMeetingEndReminders,
  meetingSettingsBusy,
  storageBusy,
  onStartModelDownload,
  onCancelModelDownload,
  onDeleteModel,
  onOpenPrivacy,
  onOpenExternalUrl,
  onOpenLegalDocument,
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

        {activeSection === "storage" && <StorageSettingsSection
          appInfo={appInfo}
          settings={settings}
          busy={storageBusy}
          onRevealFolder={onRevealFolder}
          onChooseFolder={onChooseTranscriptsFolder}
          onUseDefaultFolder={onUseDefaultTranscriptsFolder}
          onImportLegacyData={onImportLegacyData}
          onToggleRawAudio={onToggleRawAudio}
          onToggleMarkdownCopy={onToggleMarkdownCopy}
        />}

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
          {activeSection === "privacy" && (
            <PrivacyLegalSection
              onOpenExternalUrl={onOpenExternalUrl}
              onOpenLegalDocument={onOpenLegalDocument}
            />
          )}
        </div>
      </div>
    </div>
  );
}
