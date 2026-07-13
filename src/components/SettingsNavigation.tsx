import { ArrowLeft, AudioLines, CalendarDays, FileText, HardDrive, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";

export type SettingsSectionId =
  | "storage"
  | "transcription"
  | "meetings"
  | "permissions"
  | "privacy";

type SettingsNavigationProps = {
  activeSection: SettingsSectionId;
  onSelect: (id: SettingsSectionId) => void;
  onClose: () => void;
};

export function SettingsNavigation({
  activeSection,
  onSelect,
  onClose,
}: SettingsNavigationProps) {
  return (
    <nav className="settings-nav fullscreen-nav" aria-label="Settings sections">
      <button
        type="button"
        className="settings-nav-item fullscreen-nav-item fullscreen-nav-back"
        onClick={onClose}
        aria-label="Back to notes"
        title="Back to notes"
      >
        <ArrowLeft size={16} aria-hidden="true" />
      </button>
      <div className="settings-nav-title fullscreen-nav-title">Settings</div>
      <SettingsNavButton label="Storage" id="storage" icon={<HardDrive size={16} />} activeSection={activeSection} onSelect={onSelect} />
      <SettingsNavButton label="Transcription" id="transcription" icon={<AudioLines size={16} />} activeSection={activeSection} onSelect={onSelect} />
      <SettingsNavButton label="Meetings" id="meetings" icon={<CalendarDays size={16} />} activeSection={activeSection} onSelect={onSelect} />
      <SettingsNavButton label="Permissions" id="permissions" icon={<ShieldCheck size={16} />} activeSection={activeSection} onSelect={onSelect} />
      <SettingsNavButton label="Privacy & Legal" id="privacy" icon={<FileText size={16} />} activeSection={activeSection} onSelect={onSelect} />
    </nav>
  );
}

type SettingsNavButtonProps = {
  label: string;
  id: SettingsSectionId;
  icon: ReactNode;
  activeSection: SettingsSectionId;
  onSelect: (id: SettingsSectionId) => void;
};

function SettingsNavButton({ label, id, icon, activeSection, onSelect }: SettingsNavButtonProps) {
  const active = activeSection === id;
  return (
    <button
      type="button"
      className={
        active
          ? "settings-nav-item fullscreen-nav-item active"
          : "settings-nav-item fullscreen-nav-item"
      }
      onClick={() => onSelect(id)}
      aria-current={active ? "page" : undefined}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}
