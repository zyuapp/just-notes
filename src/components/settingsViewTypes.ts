import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import type { StorageUsagePayload } from "../bindings/StorageUsagePayload";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { PrivacyPane } from "../lib/permissionStatus";

export type SettingsData = {
  settings: AppSettings;
  appInfo: AppInfo | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  permissions: PermissionsPayload | null;
  meetingAccess: MeetingAccessPayload | null;
  storageUsage: StorageUsagePayload | null;
};

export type MeetingSettingsActions = {
  pendingCalendarIds: string[];
  requestingAccess: boolean;
  onRequestCalendarAccess: () => void;
  onRequestNotificationAccess: () => void;
  onToggleCalendar: (calendarId: string) => void;
  onToggleReminders: () => void;
  onSetReminderMinutes: (minutes: number) => void;
  onToggleEndReminders: () => void;
};

export type StorageActions = {
  clearing: boolean;
  onRevealFolder: () => void;
  onCopyFolderPath: () => void;
  onToggleRawAudio: () => void;
  onToggleMarkdownCopy: () => void;
  onDeleteReclaimableRawAudio: () => void;
};

export type ModelActions = {
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
  onDeleteModel: () => void;
};

export type SystemActions = {
  onOpenPrivacy: (pane: PrivacyPane) => void;
  onOpenExternalUrl: (url: string) => void;
  onOpenLegalDocument: (document: "privacy" | "notices") => void;
  onCopyVersion: () => void;
  onRevealDataFolder: () => void;
};
