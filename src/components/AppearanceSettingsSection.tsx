import { useAppearance, type AppearancePreference } from "../features/appearance/AppearanceProvider";
import { SettingsRow } from "./SettingsRow";

const options: { value: AppearancePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function AppearanceSettingsSection() {
  const { preference, setPreference } = useAppearance();
  return (
    <section>
      <SettingsRow label="Color scheme" description={preference === "system"
        ? "Matches your Mac and updates automatically."
        : `Always use ${preference} appearance.`}>
        <div className="settings-segmented appearance-options" role="group" aria-label="Color scheme">
          {options.map(({ value, label }) => (
            <button key={value} type="button" className={preference === value ? "selected" : ""}
              aria-pressed={preference === value} onClick={() => setPreference(value)}>
              {label}
            </button>
          ))}
        </div>
      </SettingsRow>
    </section>
  );
}
