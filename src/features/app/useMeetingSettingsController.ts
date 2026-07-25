import { useCallback, useRef, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import { hasWatchedCalendar, toggleCalendarSelection } from "../../lib/meetingCalendars";
import type { AppAction, AppState } from "./state";
import type { SettingsUpdater } from "./useSettingsController";

type AppDispatch = (action: AppAction) => void;

export function useMeetingSettingsController(
  state: AppState,
  dispatch: AppDispatch,
  updateSettings: (update: SettingsUpdater) => Promise<void>,
) {
  const accessRequestInFlight = useRef(false);
  const [requestingAccess, setRequestingAccess] = useState(false);
  const [pendingCalendarIds, setPendingCalendarIds] = useState<string[]>([]);
  const fail = useCallback(
    (error: unknown) => dispatch({ type: "failed", message: getApiErrorMessage(error) }),
    [dispatch],
  );

  const requestAccess = useCallback(async () => {
    // A second request would stack another system dialog, so this gesture stays
    // guarded rather than queued.
    if (accessRequestInFlight.current) return;
    accessRequestInFlight.current = true;
    setRequestingAccess(true);
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
      accessRequestInFlight.current = false;
      setRequestingAccess(false);
    }
  }, [dispatch, fail]);

  const availableCalendars = state.meetingAccess?.calendars ?? [];

  const toggleCalendar = useCallback(
    async (calendarId: string) => {
      setPendingCalendarIds((pending) => [...pending, calendarId]);
      try {
        // updateSettings serializes writes, so each updater sees the previous
        // one's result and a burst of ticks all land. The reminder preference is
        // left untouched: with no calendars selected no prompt can fire anyway,
        // and preserving it means re-selecting one restores the user's choice.
        await updateSettings((settings) => ({
          ...settings,
          meetingCalendarIds: toggleCalendarSelection(settings.meetingCalendarIds, calendarId),
        }));
      } finally {
        setPendingCalendarIds((pending) => {
          const index = pending.indexOf(calendarId);
          if (index === -1) return pending;
          return [...pending.slice(0, index), ...pending.slice(index + 1)];
        });
      }
    },
    [updateSettings],
  );

  const toggleReminders = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      meetingRemindersEnabled:
        hasWatchedCalendar(settings.meetingCalendarIds, availableCalendars)
        && !settings.meetingRemindersEnabled,
    }));
  }, [availableCalendars, updateSettings]);

  const setReminderMinutes = useCallback(
    async (meetingReminderMinutes: number) => {
      await updateSettings((settings) => ({ ...settings, meetingReminderMinutes }));
    },
    [updateSettings],
  );

  const toggleEndReminders = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      meetingEndReminders: !settings.meetingEndReminders,
    }));
  }, [updateSettings]);

  return {
    pendingCalendarIds,
    requestingAccess,
    requestAccess,
    setReminderMinutes,
    toggleCalendar,
    toggleEndReminders,
    toggleReminders,
  };
}
