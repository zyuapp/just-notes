import type { AppState } from "../features/app/state";
import type { useJustNotesController } from "../features/app/useJustNotesController";
import type { useMeetingSettingsController } from "../features/app/useMeetingSettingsController";
import type { SettingsActions } from "../features/app/useSettingsController";
import type { useStorageController } from "../features/app/useStorageController";
import type { useThreadActions } from "../features/app/useThreadActions";
import { SettingsView } from "./SettingsView";

type Props = {
  state: AppState;
  actions: ReturnType<typeof useJustNotesController>;
  meetingSettings: ReturnType<typeof useMeetingSettingsController>;
  settings: SettingsActions;
  storage: ReturnType<typeof useStorageController>;
  threads: ReturnType<typeof useThreadActions>;
};

export function AppSettingsOverlay({
  state,
  actions,
  meetingSettings,
  settings,
  storage,
  threads,
}: Props) {
  if (!state.settingsOpen || !state.settings) return null;
  return (
    <SettingsView
      data={{
        settings: state.settings,
        appInfo: state.appInfo,
        transcriptionStatus: state.transcriptionStatus,
        permissions: state.permissions,
        meetingAccess: state.meetingAccess,
        storageUsage: state.storageUsage,
      }}
      meeting={{
        pendingCalendarIds: meetingSettings.pendingCalendarIds,
        requestingAccess: meetingSettings.requestingAccess,
        onRequestCalendarAccess: () => void meetingSettings.requestCalendarAccess(),
        onRequestNotificationAccess: () => void meetingSettings.requestNotificationAccess(),
        onToggleCalendar: (id) => void meetingSettings.toggleCalendar(id),
        onToggleReminders: () => void meetingSettings.toggleReminders(),
        onSetReminderMinutes: (minutes) => void meetingSettings.setReminderMinutes(minutes),
        onToggleEndReminders: () => void meetingSettings.toggleEndReminders(),
        onToggleAutoRecord: () => void meetingSettings.toggleAutoRecord(),
        onToggleAutoStop: () => void meetingSettings.toggleAutoStop(),
        onToggleRequireAttendees: () => void meetingSettings.toggleRequireAttendees(),
      }}
      storage={{
        clearing: storage.clearing,
        onRevealFolder: () => {
          if (state.appInfo) void threads.revealPath(state.appInfo.threadsDir);
        },
        onCopyFolderPath: () => {
          if (state.appInfo) void settings.copyText(state.appInfo.threadsDir);
        },
        onToggleRawAudio: () => void settings.toggleRawAudio(),
        onToggleMarkdownCopy: () => void settings.toggleMarkdownCopy(),
        onDeleteReclaimableRawAudio: () => void storage.deleteReclaimableRawAudio(),
      }}
      model={{
        onStartModelDownload: actions.startModelDownload,
        onCancelModelDownload: () => void actions.cancelModelDownload(),
        onDeleteModel: () => void actions.deleteModel(),
      }}
      system={{
        onOpenPrivacy: (pane) => void settings.openPrivacySettings(pane),
        onOpenExternalUrl: (url) => void settings.openExternalUrl(url),
        onOpenLegalDocument: (document) => void settings.openLegalDocument(document),
        onCopyVersion: () => {
          if (state.appInfo) void settings.copyText(`Just Notes ${state.appInfo.version}`);
        },
        onRevealDataFolder: () => {
          if (state.appInfo) void threads.revealPath(state.appInfo.dataDir);
        },
      }}
      onClose={settings.closeSettings}
    />
  );
}
