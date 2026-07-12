import type { AppSettings } from "../bindings/AppSettings";
import type { AppPreferencesUpdate } from "../bindings/AppPreferencesUpdate";
import { invokeCommand } from "./transport";

export const settingsApi = {
  get(): Promise<AppSettings> {
    return invokeCommand("get_settings");
  },

  update(settings: AppSettings): Promise<AppSettings> {
    const preferences: AppPreferencesUpdate = {
      saveRawAudio: settings.saveRawAudio,
      markdownCopy: settings.markdownCopy,
      meetingRemindersEnabled: settings.meetingRemindersEnabled,
      meetingCalendarIds: settings.meetingCalendarIds,
      meetingReminderMinutes: settings.meetingReminderMinutes,
      meetingEndReminders: settings.meetingEndReminders,
    };
    return invokeCommand("update_settings", { preferences });
  },

  chooseTranscriptsFolder(): Promise<AppSettings | null> {
    return invokeCommand("choose_transcripts_folder");
  },

  importLegacyData(): Promise<AppSettings | null> {
    return invokeCommand("import_legacy_data");
  },

  useDefaultTranscriptsFolder(): Promise<AppSettings> {
    return invokeCommand("use_default_transcripts_folder");
  },
};
