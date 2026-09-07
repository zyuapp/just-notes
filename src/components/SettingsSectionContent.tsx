import { AboutSection } from "./AboutSection";
import { AppearanceSettingsSection } from "./AppearanceSettingsSection";
import { AgentAccessSettingsSection } from "./AgentAccessSettingsSection";
import { MeetingSettingsSection } from "./MeetingSettingsSection";
import { PrivacyLegalSection } from "./PrivacyLegalSection";
import { SettingsPermissionsSection } from "./SettingsPermissionsSection";
import { StorageSettingsSection } from "./StorageSettingsSection";
import { TranscriptionSettingsSection } from "./TranscriptionSettingsSection";
import type { SettingsSectionId } from "./SettingsNavigation";
import type {
  AgentAccessActions,
  MeetingSettingsActions,
  ModelActions,
  SettingsData,
  StorageActions,
  SystemActions,
} from "./settingsViewTypes";

type SettingsSectionContentProps = {
  section: SettingsSectionId;
  data: SettingsData;
  agentAccess: AgentAccessActions;
  meeting: MeetingSettingsActions;
  storage: StorageActions;
  model: ModelActions;
  system: SystemActions;
};

export function SettingsSectionContent({
  section,
  data,
  agentAccess,
  meeting,
  storage,
  model,
  system,
}: SettingsSectionContentProps) {
  switch (section) {
    case "appearance":
      return <AppearanceSettingsSection />;
    case "storage":
      return (
        <StorageSettingsSection
          appInfo={data.appInfo}
          settings={data.settings}
          usage={data.storageUsage}
          {...storage}
        />
      );
    case "agentAccess":
      return <AgentAccessSettingsSection {...agentAccess} />;
    case "transcription":
      return (
        <div id="settings-transcription">
          <TranscriptionSettingsSection
            transcriptionStatus={data.transcriptionStatus}
            {...model}
          />
        </div>
      );
    case "meetings":
      return (
        <div id="settings-meetings">
          <MeetingSettingsSection
            settings={data.settings}
            access={data.meetingAccess}
            onOpenPrivacy={system.onOpenPrivacy}
            {...meeting}
          />
        </div>
      );
    case "permissions":
      return (
        <SettingsPermissionsSection
          permissions={data.permissions}
          meetingAccess={data.meetingAccess}
          onOpenPrivacy={system.onOpenPrivacy}
        />
      );
    case "privacy":
      return (
        <PrivacyLegalSection
          onOpenExternalUrl={system.onOpenExternalUrl}
          onOpenLegalDocument={system.onOpenLegalDocument}
        />
      );
    case "about":
      return (
        <AboutSection
          appInfo={data.appInfo}
          onCopyVersion={system.onCopyVersion}
          onRevealDataFolder={system.onRevealDataFolder}
        />
      );
  }
}
