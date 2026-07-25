import type { ReactNode } from "react";

type SettingsRowProps = {
  label: string;
  description?: ReactNode;
  tone?: "normal" | "danger" | "warning";
  children?: ReactNode;
};

export function SettingsRow({ label, description, tone = "normal", children }: SettingsRowProps) {
  const hintClass = tone === "normal" ? "settings-hint" : `settings-hint settings-hint-${tone}`;
  return (
    <div className="settings-row">
      <div>
        <strong>{label}</strong>
        {description && <p className={hintClass}>{description}</p>}
      </div>
      {children && <div className="settings-row-actions">{children}</div>}
    </div>
  );
}
