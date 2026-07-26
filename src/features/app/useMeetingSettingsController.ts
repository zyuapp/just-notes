import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../api";
import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";
import { hasWatchedCalendar, toggleCalendarSelection } from "../../lib/meetingCalendars";
import { createMeetingAccessRefresh } from "./meetingAccessRefresh";
import { runMeetingAccessRequest } from "./meetingAccessRequest";
import type { AppAction, AppState } from "./state";
import { useMeetingAutomationSettings } from "./useMeetingAutomationSettings";
import type { SettingsUpdater } from "./useSettingsController";

type AppDispatch = (action: AppAction) => void;

export function useMeetingSettingsController(
  state: AppState,
  dispatch: AppDispatch,
  updateSettings: (update: SettingsUpdater) => Promise<void>,
) {
  const accessRefreshRef = useRef<ReturnType<typeof createMeetingAccessRefresh> | null>(null);
  if (accessRefreshRef.current === null) {
    accessRefreshRef.current = createMeetingAccessRefresh((meetingAccess) => {
      dispatch({ type: "meetingAccessLoaded", meetingAccess });
    });
  }
  const accessRefresh = accessRefreshRef.current;
  const [requestingAccess, setRequestingAccess] = useState(false);
  const [pendingCalendarIds, setPendingCalendarIds] = useState<string[]>([]);
  const automationSettings = useMeetingAutomationSettings(updateSettings);

  const refreshAccess = useCallback(async () => {
    try {
      await accessRefresh.refresh(api.meetings.getAccessStatus);
    } catch {
      // Automatic permission refreshes are best-effort; explicit requests report failures.
    }
  }, [accessRefresh]);

  // The native window focus event, not the DOM one: WKWebView does not reliably
  // re-fire `focus` on the page when the app window regains key status after a
  // system permission dialog or a trip to System Settings.
  useEffect(() => {
    if (!state.settingsOpen) return;
    void refreshAccess();

    let disposed = false;
    let unlisten: (() => void) | undefined;
    void api.window
      .onFocusChanged((focused) => {
        if (focused && !disposed) void refreshAccess();
      })
      .then((stopListening) => {
        if (disposed) stopListening();
        else unlisten = stopListening;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      accessRefresh.invalidate();
      unlisten?.();
    };
  }, [accessRefresh, refreshAccess, state.settingsOpen]);

  const runAccessRequest = useCallback(
    async (request: () => Promise<MeetingAccessPayload>) => {
      // A second request would stack another system dialog, so this gesture stays
      // guarded rather than queued.
      if (!accessRefresh.beginPermissionRequest()) return;
      setRequestingAccess(true);
      try {
        await runMeetingAccessRequest({
          dispatch,
          request,
          refresh: api.meetings.getAccessStatus,
        });
      } finally {
        void accessRefresh.endPermissionRequest().catch(() => undefined);
        setRequestingAccess(false);
      }
    },
    [accessRefresh, dispatch],
  );

  const requestCalendarAccess = useCallback(
    () => runAccessRequest(() => api.meetings.requestCalendarAccess()),
    [runAccessRequest],
  );

  const requestNotificationAccess = useCallback(
    () => runAccessRequest(() => api.meetings.requestNotificationAccess()),
    [runAccessRequest],
  );

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
    ...automationSettings,
    pendingCalendarIds,
    requestingAccess,
    requestCalendarAccess,
    requestNotificationAccess,
    setReminderMinutes,
    toggleCalendar,
    toggleEndReminders,
    toggleReminders,
  };
}
