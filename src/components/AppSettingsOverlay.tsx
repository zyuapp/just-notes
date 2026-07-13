import type { AppState } from "../features/app/state";
import type { useJustNotesController } from "../features/app/useJustNotesController";
import type { useMeetingSettingsController } from "../features/app/useMeetingSettingsController";
import type { SettingsActions } from "../features/app/useSettingsController";
import type { useThreadActions } from "../features/app/useThreadActions";
import { SettingsView } from "./SettingsView";

type Props = {
  state: AppState;
  actions: ReturnType<typeof useJustNotesController>;
  meetingSettings: ReturnType<typeof useMeetingSettingsController>;
  settings: SettingsActions;
  threads: ReturnType<typeof useThreadActions>;
  onImportLegacyData: () => void;
};

export function AppSettingsOverlay({
  state,
  actions,
  meetingSettings,
  settings,
  threads,
  onImportLegacyData,
}: Props) {
  if (!state.settingsOpen || !state.settings) return null;
  return <SettingsView
    settings={state.settings}
    appInfo={state.appInfo}
    transcriptionStatus={state.transcriptionStatus}
    permissions={state.permissions}
    meetingAccess={state.meetingAccess}
    meetingSettingsBusy={meetingSettings.busy}
    storageBusy={state.recorderState !== "idle" || state.finalization?.state === "running"}
    onClose={settings.closeSettings}
    onRevealFolder={() => {
      if (state.appInfo) void threads.revealPath(state.appInfo.threadsDir);
    }}
    onImportLegacyData={onImportLegacyData}
    onToggleRawAudio={() => void settings.toggleRawAudio()}
    onToggleMarkdownCopy={() => void settings.toggleMarkdownCopy()}
    onRequestMeetingAccess={() => void meetingSettings.requestAccess()}
    onToggleMeetingCalendar={(id) => void meetingSettings.toggleCalendar(id)}
    onToggleMeetingReminders={() => void meetingSettings.toggleReminders()}
    onSetMeetingReminderMinutes={(minutes) => void meetingSettings.setReminderMinutes(minutes)}
    onToggleMeetingEndReminders={() => void meetingSettings.toggleEndReminders()}
    onCancelModelDownload={() => void actions.cancelModelDownload()}
    onStartModelDownload={actions.startModelDownload}
    onDeleteModel={() => void actions.deleteModel()}
    onOpenPrivacy={(pane) => void settings.openPrivacySettings(pane)}
    onOpenExternalUrl={(url) => void settings.openExternalUrl(url)}
    onOpenLegalDocument={(document) => void settings.openLegalDocument(document)}
  />;
}
