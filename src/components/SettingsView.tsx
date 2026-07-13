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
import { FullscreenView } from "./FullscreenView";

const SECTION_COPY: Record<SettingsSectionId, { title: string; description: string }> = {
  storage: { title: "Storage", description: "See where recordings live and choose which files are kept." },
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
  onImportLegacyData: () => void;
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
  onImportLegacyData,
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
  return (
    <FullscreenView
      ariaLabel="Settings"
      navigation={
        <SettingsNavigation
          activeSection={activeSection}
          onSelect={setActiveSection}
          onClose={onClose}
        />
      }
      title={activeSectionCopy.title}
      description={activeSectionCopy.description}
      panelClassName="settings-panel"
      onClose={onClose}
    >
      {activeSection === "storage" && (
        <StorageSettingsSection
          appInfo={appInfo}
          settings={settings}
          busy={storageBusy}
          onRevealFolder={onRevealFolder}
          onImportLegacyData={onImportLegacyData}
          onToggleRawAudio={onToggleRawAudio}
          onToggleMarkdownCopy={onToggleMarkdownCopy}
        />
      )}

      {activeSection === "transcription" && (
        <div id="settings-transcription">
          <TranscriptionSettingsSection
            transcriptionStatus={transcriptionStatus}
            onStartModelDownload={onStartModelDownload}
            onCancelModelDownload={onCancelModelDownload}
            onDeleteModel={onDeleteModel}
          />
        </div>
      )}

      {activeSection === "meetings" && (
        <div id="settings-meetings">
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
        </div>
      )}

      {activeSection === "permissions" && (
        <SettingsPermissionsSection permissions={permissions} onOpenPrivacy={onOpenPrivacy} />
      )}
      {activeSection === "privacy" && (
        <PrivacyLegalSection
          onOpenExternalUrl={onOpenExternalUrl}
          onOpenLegalDocument={onOpenLegalDocument}
        />
      )}
    </FullscreenView>
  );
}
