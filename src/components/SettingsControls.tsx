type SettingsToggleProps = {
  label: string;
  description: string;
  checked: boolean;
  onToggle: () => void;
  disabled?: boolean;
};

export function SettingsToggle({
  label,
  description,
  checked,
  onToggle,
  disabled = false,
}: SettingsToggleProps) {
  return (
    <div className="settings-row">
      <div>
        <strong>{label}</strong>
        <p className="settings-hint">{description}</p>
      </div>
      <label className="settings-switch">
        <input
          type="checkbox"
          checked={checked}
          onChange={onToggle}
          aria-label={label}
          disabled={disabled}
        />
        <span className="settings-knob" aria-hidden="true" />
      </label>
    </div>
  );
}
