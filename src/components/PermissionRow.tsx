import { permissionDisplay, type PrivacyPane } from "../lib/permissionStatus";
import type { StatusTone } from "../lib/statusTone";
import { Button } from "./Button";
import { SettingsRow } from "./SettingsRow";
import { StatusPill } from "./StatusPill";

type PermissionRowProps = {
  label: string;
  status: string | undefined;
  grantedHint: string;
  blockedHint: string;
  // Shown before macOS has asked. Not a problem state, so it stays neutral.
  pendingHint: string;
  pane: PrivacyPane;
  onOpenPrivacy: (pane: PrivacyPane) => void;
};

export function PermissionRow({
  label,
  status,
  grantedHint,
  blockedHint,
  pendingHint,
  pane,
  onOpenPrivacy,
}: PermissionRowProps) {
  const display = permissionDisplay(status);
  const { description, tone } = rowCopy(display.tone, { grantedHint, blockedHint, pendingHint });
  return (
    <SettingsRow label={label} description={description} tone={tone}>
      <StatusPill tone={display.tone} label={display.pill} />
      <Button
        size="compact"
        onClick={() => onOpenPrivacy(pane)}
        aria-label={`Open System Settings for ${label}`}
      >
        Open System Settings
      </Button>
    </SettingsRow>
  );
}

function rowCopy(
  tone: StatusTone,
  hints: { grantedHint: string; blockedHint: string; pendingHint: string },
): { description: string; tone: "normal" | "danger" | "warning" } {
  if (tone === "ok") return { description: hints.grantedHint, tone: "normal" };
  if (tone === "idle") return { description: hints.pendingHint, tone: "normal" };
  if (tone === "danger") return { description: hints.blockedHint, tone: "danger" };
  return { description: hints.blockedHint, tone: "warning" };
}
