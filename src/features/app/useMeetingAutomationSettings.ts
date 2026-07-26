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

  const toggleAutoStop = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      meetingAutoStopEnabled: !settings.meetingAutoStopEnabled,
    }));
  }, [updateSettings]);

  const toggleRequireAttendees = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      meetingAutoRecordRequiresAttendees: !settings.meetingAutoRecordRequiresAttendees,
    }));
  }, [updateSettings]);

  return { toggleAutoRecord, toggleAutoStop, toggleRequireAttendees };
}
