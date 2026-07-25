import { useCallback, useEffect } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { AppAction, AppState } from "./state";
import { runRecordingStart } from "./recordingStart";
import { useTranscriptionModelController } from "./useTranscriptionModelController";

type AppDispatch = (action: AppAction) => void;

export function useJustNotesController(state: AppState, dispatch: AppDispatch) {
  const modelActions = useTranscriptionModelController(state, dispatch);
  const selectThread = useCallback(
    async (threadId: string) => {
      dispatch({ type: "errorCleared" });
      try {
        dispatch({ type: "threadSelected", detail: await api.threads.get(threadId) });
      } catch (error) {
        dispatch({ type: "failed", message: getApiErrorMessage(error) });
      }
    },
    [dispatch],
  );

  const refreshThreads = useCallback(
    async (nextSelectedId?: string) => {
      try {
        const threads = await api.threads.list();
        dispatch({ type: "threadsLoaded", threads });
        const threadId = nextSelectedId ?? state.selectedThreadId ?? threads[0]?.id ?? null;
        if (threadId && threads.some((thread) => thread.id === threadId)) {
          await selectThread(threadId);
        }
      } catch (error) {
        dispatch({ type: "failed", message: getApiErrorMessage(error) });
      }
    },
    [dispatch, selectThread, state.selectedThreadId],
  );

  const bootstrap = useCallback(async () => {
    dispatch({ type: "errorCleared" });
    try {
      const [info, transcriptionStatus, threads, settings, permissions, meetingAccess] = await Promise.all([
        api.app.getInfo(),
        api.transcription.getStatus(),
        api.threads.list(),
        api.settings.get(),
        api.system.getPermissions(),
        api.meetings.getAccessStatus(),
      ]);
      dispatch({
        type: "bootstrapLoaded",
        info,
        transcriptionStatus,
        threads,
        settings,
        permissions,
        meetingAccess,
      });
      if (threads.length > 0) {
        await selectThread(threads[0].id);
      }
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    }
  }, [dispatch, selectThread]);

  const createThread = useCallback(async () => {
    dispatch({ type: "errorCleared" });
    try {
      const detail = await api.threads.create();
      dispatch({ type: "threadSelected", detail });
      await refreshThreads(detail.summary.id);
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    }
  }, [dispatch, refreshThreads]);

  const startRecording = useCallback(async () => {
    await runRecordingStart({
      dispatch,
      start: () => api.recording.start(state.selectedThreadId),
      refreshThreads,
    });
  }, [dispatch, refreshThreads, state.selectedThreadId]);

  const startFixtureRecording = useCallback(async () => {
    await runRecordingStart({
      dispatch,
      start: () => api.recording.startFixture(state.selectedThreadId),
      refreshThreads,
    });
  }, [dispatch, refreshThreads, state.selectedThreadId]);

  const stopRecording = useCallback(async () => {
    dispatch({ type: "recordingStopping" });
    try {
      const detail = await api.recording.stop();
      dispatch({ type: "recordingStopped", detail });
      await refreshThreads(detail.summary.id);
    } catch (error) {
      dispatch({ type: "recordingStopFailed", message: getApiErrorMessage(error) });
    }
  }, [dispatch, refreshThreads]);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  return {
    bootstrap,
    createThread,
    refreshThreads,
    selectThread,
    startFixtureRecording,
    startRecording,
    stopRecording,
    ...modelActions,
  };
}
