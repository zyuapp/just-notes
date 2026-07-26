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

  return { toggleAutoRecord };
}
