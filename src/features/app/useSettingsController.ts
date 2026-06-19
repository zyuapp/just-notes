import { useCallback } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { AppSettings } from "../../bindings/AppSettings";
import type { AppAction, AppState } from "./state";

type AppDispatch = (action: AppAction) => void;

export type SettingsActions = ReturnType<typeof useSettingsController>;

export function useSettingsController(
  state: AppState,
  dispatch: AppDispatch,
  onStorageChanged: () => Promise<void>,
) {
  const fail = useCallback(
    (error: unknown) => dispatch({ type: "failed", message: getApiErrorMessage(error) }),
    [dispatch],
  );

  const openSettings = useCallback(() => {
    dispatch({ type: "settingsOpenChanged", open: true });
    api.system
      .getPermissions()
      .then((permissions) => dispatch({ type: "permissionsLoaded", permissions }))
      .catch(() => undefined);
  }, [dispatch]);

  const closeSettings = useCallback(
    () => dispatch({ type: "settingsOpenChanged", open: false }),
    [dispatch],
  );

  const saveSettings = useCallback(
    async (settings: AppSettings, refreshAfterSave: boolean) => {
      try {
        const saved = await api.settings.update(settings);
        dispatch({ type: "settingsLoaded", settings: saved });
        if (refreshAfterSave) {
          await onStorageChanged();
        }
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail, onStorageChanged],
  );

  const toggleRawAudio = useCallback(async () => {
    if (!state.settings) return;
    await saveSettings({ ...state.settings, saveRawAudio: !state.settings.saveRawAudio }, false);
  }, [saveSettings, state.settings]);

  const toggleMarkdownCopy = useCallback(async () => {
    if (!state.settings) return;
    await saveSettings({ ...state.settings, markdownCopy: !state.settings.markdownCopy }, false);
  }, [saveSettings, state.settings]);

  const chooseTranscriptsFolder = useCallback(async () => {
    if (!state.settings) return;
    try {
      const folder = await api.settings.pickFolder();
      if (folder) {
        await saveSettings({ ...state.settings, transcriptsDir: folder }, true);
      }
    } catch (error) {
      fail(error);
    }
  }, [fail, saveSettings, state.settings]);

  const useDefaultFolder = useCallback(async () => {
    if (!state.settings) return;
    await saveSettings({ ...state.settings, transcriptsDir: null }, true);
  }, [saveSettings, state.settings]);

  const openPrivacySettings = useCallback(
    async (pane: "microphone" | "system-audio") => {
      try {
        await api.system.openPrivacySettings(pane);
      } catch (error) {
        fail(error);
      }
    },
    [fail],
  );

  return {
    chooseTranscriptsFolder,
    closeSettings,
    openPrivacySettings,
    openSettings,
    toggleMarkdownCopy,
    toggleRawAudio,
    useDefaultFolder,
  };
}
