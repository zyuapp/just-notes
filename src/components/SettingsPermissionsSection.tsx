import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import { Button } from "./Button";
import { permissionLabel } from "./SettingsControls";

type PrivacyPane = "microphone" | "system-audio" | "calendar" | "notifications";

type SettingsPermissionsSectionProps = {
  permissions: PermissionsPayload | null;
  onOpenPrivacy: (pane: PrivacyPane) => void;
};

export function SettingsPermissionsSection({
  permissions,
  onOpenPrivacy,
}: SettingsPermissionsSectionProps) {
  return (
    <section id="settings-permissions">
      <PermissionRow
        label="Microphone"
        description={permissionLabel(permissions?.microphone)}
        onOpen={() => onOpenPrivacy("microphone")}
      />
      <PermissionRow
        label="System audio"
        description="Managed under Screen & System Audio Recording in macOS."
        onOpen={() => onOpenPrivacy("system-audio")}
      />
    </section>
  );
}

type PermissionRowProps = { label: string; description: string; onOpen: () => void };

function PermissionRow({ label, description, onOpen }: PermissionRowProps) {
  return (
    <div className="settings-row">
      <div>
        <strong>{label}</strong>
        <p className="settings-hint">{description}</p>
      </div>
      <div className="settings-row-actions">
        <Button size="compact" onClick={onOpen}>Open System Settings</Button>
      </div>
    </div>
  );
}
