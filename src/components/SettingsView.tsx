import { useState } from "react";
import { settingsAttention } from "../lib/permissionStatus";
import { FullscreenView } from "./FullscreenView";
import { SettingsNavigation, type SettingsSectionId } from "./SettingsNavigation";
import { SettingsSectionContent } from "./SettingsSectionContent";
import type {
  AgentAccessActions,
  MeetingSettingsActions,
  ModelActions,
  SettingsData,
  StorageActions,
  SystemActions,
} from "./settingsViewTypes";

const SECTION_COPY: Record<SettingsSectionId, { title: string; description: string }> = {
  appearance: { title: "Appearance", description: "Use your Mac’s appearance or choose light or dark." },
  storage: { title: "Storage", description: "See where recordings live and choose which files are kept." },
  agentAccess: { title: "Agent Access", description: "Install read-only guides for local coding agents." },
  transcription: { title: "Transcription", description: "Process every recording locally on this Mac." },
  meetings: { title: "Meetings", description: "Choose when calendar meetings prompt or record automatically." },
  permissions: { title: "Permissions", description: "Review the system access Just Notes uses." },
  privacy: { title: "Privacy & Legal", description: "See how local data and third-party software are handled." },
  about: { title: "About", description: "Version and storage details for this copy of Just Notes." },
};

type SettingsViewProps = {
  data: SettingsData;
  agentAccess: AgentAccessActions;
  meeting: MeetingSettingsActions;
  storage: StorageActions;
  model: ModelActions;
  system: SystemActions;
  onClose: () => void;
};

export function SettingsView({ data, agentAccess, meeting, storage, model, system, onClose }: SettingsViewProps) {
  const [activeSection, setActiveSection] = useState<SettingsSectionId>("appearance");
  const activeSectionCopy = SECTION_COPY[activeSection];
  const attention = settingsAttention(data.permissions, data.meetingAccess);

  return (
    <FullscreenView
      ariaLabel="Settings"
      navigation={
        <SettingsNavigation
          activeSection={activeSection}
          attention={attention}
          onSelect={setActiveSection}
          onClose={onClose}
        />
      }
      title={activeSectionCopy.title}
      description={activeSectionCopy.description}
      panelClassName="settings-panel"
      onClose={onClose}
    >
      <SettingsSectionContent
        section={activeSection}
        data={data}
        agentAccess={agentAccess}
        meeting={meeting}
        storage={storage}
        model={model}
        system={system}
      />
    </FullscreenView>
  );
}
