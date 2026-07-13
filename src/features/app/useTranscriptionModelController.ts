import { useCallback, useEffect, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { TranscriptionModelStatus } from "../../bindings/TranscriptionModelStatus";
import { isModelDownloadActive } from "../../lib/transcriptionModel";
import type { AppAction, AppState } from "./state";

type AppDispatch = (action: AppAction) => void;

export function useTranscriptionModelController(state: AppState, dispatch: AppDispatch) {
  const [modelDownloadConsent, setModelDownloadConsent] = useState<TranscriptionModelStatus | null>(null);
  const refreshTranscriptionStatus = useCallback(async () => {
    try {
      const [transcriptionStatus, settings] = await Promise.all([
        api.transcription.getStatus(),
        api.settings.get(),
      ]);
      dispatch({ type: "transcriptionStatusLoaded", transcriptionStatus });
      dispatch({ type: "settingsLoaded", settings });
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    }
  }, [dispatch]);

  const startModelDownload = useCallback(() => {
    const selectedModel = state.transcriptionStatus?.availableModels.find((model) => model.selected);
    if (selectedModel) setModelDownloadConsent(selectedModel);
  }, [state.transcriptionStatus]);

  const confirmModelDownload = useCallback(async () => {
    setModelDownloadConsent(null);
    try {
      const transcriptionStatus = await api.transcription.startModelDownload();
      dispatch({ type: "errorCleared" });
      dispatch({ type: "transcriptionStatusLoaded", transcriptionStatus });
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    }
  }, [dispatch]);

  const dismissModelDownloadConsent = useCallback(() => setModelDownloadConsent(null), []);

  const cancelModelDownload = useCallback(async () => {
    dispatch({ type: "errorCleared" });
    try {
      const transcriptionStatus = await api.transcription.cancelModelDownload();
      dispatch({ type: "transcriptionStatusLoaded", transcriptionStatus });
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    }
  }, [dispatch]);

  const deleteModel = useCallback(async () => {
    dispatch({ type: "errorCleared" });
    try {
      const transcriptionStatus = await api.transcription.deleteModel();
      dispatch({ type: "transcriptionStatusLoaded", transcriptionStatus });
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    }
  }, [dispatch]);

  useEffect(() => {
    const selectedModel = state.transcriptionStatus?.availableModels.find((model) => model.selected);
    if (!selectedModel || !isModelDownloadActive(selectedModel)) return undefined;

    const timer = window.setTimeout(() => {
      void refreshTranscriptionStatus();
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [refreshTranscriptionStatus, state.transcriptionStatus]);

  return {
    cancelModelDownload,
    confirmModelDownload,
    deleteModel,
    dismissModelDownloadConsent,
    modelDownloadConsent,
    refreshTranscriptionStatus,
    startModelDownload,
  };
}
