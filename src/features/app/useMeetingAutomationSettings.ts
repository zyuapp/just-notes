import { useCallback } from "react";
import type { SettingsUpdater } from "./useSettingsController";

export function useMeetingAutomationSettings(
  updateSettings: (update: SettingsUpdater) => Promise<void>,
) {
  const toggleAutoRecord = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      meetingAutoRecordEnabled: !settings.meetingAutoRecordEnabled,
    }));
  }, [updateSettings]);

  const toggleJoinReminders = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      meetingJoinRemindersEnabled: !settings.meetingJoinRemindersEnabled,
    }));
  }, [updateSettings]);

  const toggleCallDetection = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      callDetectionEnabled: !settings.callDetectionEnabled,
    }));
  }, [updateSettings]);

  const resetIgnoredCallApps = useCallback(async () => {
    await updateSettings((settings) => ({ ...settings, callDetectionIgnoredApps: [] }));
  }, [updateSettings]);

  return { toggleAutoRecord, toggleJoinReminders, toggleCallDetection, resetIgnoredCallApps };
}
