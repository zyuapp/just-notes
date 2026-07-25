import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import type { StatusTone } from "./statusTone";

export type PrivacyPane = "microphone" | "system-audio" | "calendar" | "notifications";

export type PermissionDisplay = {
  tone: StatusTone;
  pill: string;
  // True when the permission blocks or degrades a feature the user has enabled.
  needsAttention: boolean;
};

export function permissionDisplay(status: string | undefined): PermissionDisplay {
  switch (status) {
    case "authorized":
      return { tone: "ok", pill: "Granted", needsAttention: false };
    case "denied":
      return { tone: "danger", pill: "Denied", needsAttention: true };
    case "restricted":
      return { tone: "danger", pill: "Restricted", needsAttention: true };
    case "notDetermined":
      return { tone: "idle", pill: "Not asked yet", needsAttention: false };
    // macOS only exposes a boolean preflight for system audio, so a first run
    // and an explicit refusal are indistinguishable.
    case "notGranted":
      return { tone: "warning", pill: "Not granted", needsAttention: true };
    default:
      return { tone: "idle", pill: "Unknown", needsAttention: false };
  }
}

// Meeting prompts need the calendar the user connected and a way to show
// themselves; either one missing means nothing can fire.
export function meetingPromptsBlocked(access: MeetingAccessPayload | null): boolean {
  if (access?.calendarAuthorization !== "authorized") return false;
  return permissionDisplay(access.notificationAuthorization).tone !== "ok";
}

// The single source for which settings sections are advertising a problem, so a
// nav badge can never disagree with the banner it points at.
export function settingsAttention(
  permissions: PermissionsPayload | null,
  meetingAccess: MeetingAccessPayload | null,
): { permissions: boolean; meetings: boolean } {
  return {
    permissions: recordingBlockers(permissions).length > 0,
    meetings: meetingPromptsBlocked(meetingAccess),
  };
}

// The permissions recording depends on, in the order the Permissions pane lists
// them, filtered to the ones currently standing in the way.
export function recordingBlockers(permissions: PermissionsPayload | null): string[] {
  return [
    { label: "Microphone", status: permissions?.microphone },
    { label: "System audio", status: permissions?.systemAudio },
  ]
    .filter((entry) => permissionDisplay(entry.status).needsAttention)
    .map((entry) => entry.label);
}
