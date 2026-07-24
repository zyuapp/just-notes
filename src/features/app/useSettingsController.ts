import { useCallback, useRef } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { AppSettings } from "../../bindings/AppSettings";
import type { PrivacyPane } from "../../lib/permissionStatus";
import type { AppAction, AppState } from "./state";

type AppDispatch = (action: AppAction) => void;

export type SettingsActions = ReturnType<typeof useSettingsController>;
export type SettingsUpdater = (settings: AppSettings) => AppSettings;

export function useSettingsController(
  state: AppState,
  dispatch: AppDispatch,
  onSettingsChanged: () => Promise<void>,
) {
  const settingsRef = useRef(state.settings);
  const updateQueue = useRef<Promise<void>>(Promise.resolve());
  settingsRef.current = state.settings;
  const fail = useCallback(
    (error: unknown) => dispatch({ type: "failed", message: getApiErrorMessage(error) }),
    [dispatch],
  );

  // Permissions change outside the app, in System Settings, so anything that
  // returns the user's attention to Just Notes has to re-read them.
  const refreshPermissions = useCallback(() => {
    api.system
      .getPermissions()
      .then((permissions) => dispatch({ type: "permissionsLoaded", permissions }))
      .catch(() => undefined);
  }, [dispatch]);

  // Opening only flips the flag; whoever renders the overlay keeps its data
  // fresh, so a permission granted mid-session is picked up on the way back.
  const openSettings = useCallback(
    () => dispatch({ type: "settingsOpenChanged", open: true }),
    [dispatch],
  );

  const closeSettings = useCallback(
    () => dispatch({ type: "settingsOpenChanged", open: false }),
    [dispatch],
  );
  const updateSettings = useCallback(
    (update: SettingsUpdater, refreshAfterSave = false) => {
      const operation = updateQueue.current.then(async () => {
        const current = settingsRef.current;
        if (!current) return;
        const saved = await api.settings.update(update(current));
        settingsRef.current = saved;
        dispatch({ type: "settingsLoaded", settings: saved });
        if (refreshAfterSave) await onSettingsChanged();
      });
      updateQueue.current = operation.catch(() => undefined);
      return operation.catch((error) => {
        fail(error);
      });
    },
    [dispatch, fail, onSettingsChanged],
  );

  const toggleRawAudio = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      saveRawAudio: !settings.saveRawAudio,
    }));
  }, [updateSettings]);

  const toggleMarkdownCopy = useCallback(async () => {
    await updateSettings((settings) => ({
      ...settings,
      markdownCopy: !settings.markdownCopy,
    }));
  }, [updateSettings]);

  const openPrivacySettings = useCallback(
    async (pane: PrivacyPane) => {
      try {
        await api.system.openPrivacySettings(pane);
      } catch (error) {
        fail(error);
      }
    },
    [fail],
  );

  const copyText = useCallback(
    async (text: string) => {
      try {
        await api.system.copyText(text);
      } catch (error) {
        fail(error);
      }
    },
    [fail],
  );

  const openExternalUrl = useCallback(async (url: string) => {
    try {
      await api.system.openExternalUrl(url);
    } catch (error) {
      fail(error);
    }
  }, [fail]);

  const openLegalDocument = useCallback(async (document: "privacy" | "notices") => {
    try {
      await api.system.openLegalDocument(document);
    } catch (error) {
      fail(error);
    }
  }, [fail]);

  return {
    closeSettings,
    copyText,
    openExternalUrl,
    openLegalDocument,
    openPrivacySettings,
    openSettings,
    refreshPermissions,
    toggleMarkdownCopy,
    toggleRawAudio,
    updateSettings,
  };
}
