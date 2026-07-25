import {
  ArrowLeft,
  AudioLines,
  CalendarDays,
  FileText,
  HardDrive,
  Info,
  ShieldCheck,
} from "lucide-react";
import type { ReactNode } from "react";

export type SettingsSectionId =
  | "storage"
  | "transcription"
  | "meetings"
  | "permissions"
  | "privacy"
  | "about";

type SettingsNavigationProps = {
  activeSection: SettingsSectionId;
  attention: Partial<Record<SettingsSectionId, boolean>>;
  onSelect: (id: SettingsSectionId) => void;
  onClose: () => void;
};

const SECTIONS: { id: SettingsSectionId; label: string; icon: ReactNode }[] = [
  { id: "storage", label: "Storage", icon: <HardDrive size={16} /> },
  { id: "transcription", label: "Transcription", icon: <AudioLines size={16} /> },
  { id: "meetings", label: "Meetings", icon: <CalendarDays size={16} /> },
  { id: "permissions", label: "Permissions", icon: <ShieldCheck size={16} /> },
  { id: "privacy", label: "Privacy & Legal", icon: <FileText size={16} /> },
  { id: "about", label: "About", icon: <Info size={16} /> },
];

export function SettingsNavigation({
  activeSection,
  attention,
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
      {SECTIONS.map((section) => (
        <SettingsNavButton
          key={section.id}
          label={section.label}
          id={section.id}
          icon={section.icon}
          needsAttention={attention[section.id] ?? false}
          activeSection={activeSection}
          onSelect={onSelect}
        />
      ))}
    </nav>
  );
}

type SettingsNavButtonProps = {
  label: string;
  id: SettingsSectionId;
  icon: ReactNode;
  needsAttention: boolean;
  activeSection: SettingsSectionId;
  onSelect: (id: SettingsSectionId) => void;
};

function SettingsNavButton({
  label,
  id,
  icon,
  needsAttention,
  activeSection,
  onSelect,
}: SettingsNavButtonProps) {
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
      {needsAttention && (
        <span className="settings-nav-dot" role="img" aria-label={`${label} needs attention`} />
      )}
    </button>
  );
}
