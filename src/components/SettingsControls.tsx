type SettingsToggleProps = {
  label: string;
  description: string;
  checked: boolean;
  onToggle: () => void;
};

export function SettingsToggle({ label, description, checked, onToggle }: SettingsToggleProps) {
  return (
    <div className="settings-row">
      <div>
        <strong>{label}</strong>
        <p className="settings-hint">{description}</p>
      </div>
      <label className="settings-switch">
        <input type="checkbox" checked={checked} onChange={onToggle} aria-label={label} />
        <span className="settings-knob" aria-hidden="true" />
      </label>
    </div>
  );
}

export function permissionLabel(status: string | undefined): string {
  switch (status) {
    case "authorized":
      return "Access granted";
    case "denied":
      return "Access denied — enable it in System Settings";
    case "restricted":
      return "Restricted by macOS policy";
    case "notDetermined":
      return "macOS will ask when you start a recording";
    default:
      return "Status unknown";
  }
}
