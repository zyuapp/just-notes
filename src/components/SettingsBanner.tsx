import { AlertTriangle } from "lucide-react";
import type { ReactNode } from "react";

type SettingsBannerProps = {
  tone: "warning" | "danger";
  title: string;
  description: string;
  children?: ReactNode;
};

// Attention-level state that would be missed as another grey row: a permission
// that blocks recording, or a feature configured but unable to fire.
export function SettingsBanner({ tone, title, description, children }: SettingsBannerProps) {
  return (
    <div className={`settings-banner settings-banner-${tone}`} role="status">
      <AlertTriangle size={16} aria-hidden="true" />
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
      {children && <div className="settings-row-actions">{children}</div>}
    </div>
  );
}
