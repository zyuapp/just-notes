import { useCallback, useEffect, useRef, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { AppAction, AppState } from "./state";
import type { SettingsUpdater } from "./useSettingsController";

type AppDispatch = (action: AppAction) => void;

export function useMeetingSettingsController(
  state: AppState,
  dispatch: AppDispatch,
  updateSettings: (update: SettingsUpdater) => Promise<void>,
) {
  const updateInFlight = useRef(false);
  const [busy, setBusy] = useState(false);
  const fail = useCallback(
    (error: unknown) => dispatch({ type: "failed", message: getApiErrorMessage(error) }),
    [dispatch],
  );

  const requestAccess = useCallback(async () => {
    if (updateInFlight.current) return;
    updateInFlight.current = true;
    setBusy(true);
    dispatch({ type: "errorCleared" });
    try {
      const meetingAccess = await api.meetings.requestAccess();
      dispatch({ type: "meetingAccessLoaded", meetingAccess });
    } catch (error) {
      api.meetings
        .getAccessStatus()
        .then((meetingAccess) => dispatch({ type: "meetingAccessLoaded", meetingAccess }))
        .catch(() => undefined);
      fail(error);
    } finally {
      updateInFlight.current = false;
      setBusy(false);
    }
  }, [dispatch, fail]);

  const persist = useCallback(
    async (update: SettingsUpdater) => {
      if (updateInFlight.current) return;
      updateInFlight.current = true;
      setBusy(true);
      try {
        await updateSettings(update);
      } finally {
        updateInFlight.current = false;
        setBusy(false);
      }
    },
    [updateSettings],
  );

  const toggleCalendar = useCallback(
    async (calendarId: string) => {
      const available = new Set(
        state.meetingAccess?.calendars.map((calendar) => calendar.id) ?? [],
      );
      await persist((settings) => {
        const selected = new Set(settings.meetingCalendarIds.filter((id) => available.has(id)));
        const hadValidSelection = selected.size > 0;
        if (selected.has(calendarId)) selected.delete(calendarId);
        else selected.add(calendarId);
        return {
          ...settings,
          meetingCalendarIds: [...selected],
          meetingRemindersEnabled:
            selected.size > 0 && hadValidSelection && settings.meetingRemindersEnabled,
        };
      });
    },
    [persist, state.meetingAccess?.calendars],
  );

  const toggleReminders = useCallback(async () => {
    const available = new Set(state.meetingAccess?.calendars.map((calendar) => calendar.id) ?? []);
    await persist((settings) => ({
      ...settings,
      meetingRemindersEnabled:
        settings.meetingCalendarIds.some((id) => available.has(id))
          && !settings.meetingRemindersEnabled,
    }));
  }, [persist, state.meetingAccess?.calendars]);

  const setReminderMinutes = useCallback(
    async (meetingReminderMinutes: number) => {
      await persist((settings) => ({ ...settings, meetingReminderMinutes }));
    },
    [persist],
  );

  const toggleEndReminders = useCallback(async () => {
    await persist((settings) => ({
      ...settings,
      meetingEndReminders: !settings.meetingEndReminders,
    }));
  }, [persist]);

  useEffect(() => {
    if (!state.settings || state.meetingAccess?.calendarAuthorization !== "authorized") return;
    const available = new Set(state.meetingAccess.calendars.map((calendar) => calendar.id));
    const selected = state.settings.meetingCalendarIds.filter((id) => available.has(id));
    if (selected.length === state.settings.meetingCalendarIds.length) return;
    void updateSettings((settings) => {
      const meetingCalendarIds = settings.meetingCalendarIds.filter((id) => available.has(id));
      return {
        ...settings,
        meetingCalendarIds,
        meetingRemindersEnabled:
          meetingCalendarIds.length > 0 && settings.meetingRemindersEnabled,
      };
    });
  }, [state.meetingAccess, state.settings, updateSettings]);

  return {
    busy,
    requestAccess,
    setReminderMinutes,
    toggleCalendar,
    toggleEndReminders,
    toggleReminders,
  };
}
