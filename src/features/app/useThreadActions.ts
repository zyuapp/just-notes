import { useCallback } from "react";
import { api, getApiErrorMessage } from "../../api";
import { formatDuration } from "../../lib/format";
import { transcriptToText } from "../../lib/transcript";
import type { AppAction, AppState } from "./state";

type AppDispatch = (action: AppAction) => void;

export type ThreadActions = ReturnType<typeof useThreadActions>;

export function useThreadActions(
  state: AppState,
  dispatch: AppDispatch,
  refreshThreads: (nextSelectedId?: string) => Promise<void>,
) {
  const selected = state.selectedThread;
  const threadId = selected?.summary.id ?? null;

  const fail = useCallback(
    (error: unknown) => dispatch({ type: "failed", message: getApiErrorMessage(error) }),
    [dispatch],
  );

  const renameThread = useCallback(
    async (title: string) => {
      if (!threadId) return;
      try {
        dispatch({ type: "threadUpdated", detail: await api.threads.rename(threadId, title) });
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail, threadId],
  );

  const deleteThread = useCallback(async () => {
    if (!threadId) return;
    try {
      await api.threads.delete(threadId);
      dispatch({ type: "threadDeleted", threadId });
      await refreshThreads();
    } catch (error) {
      fail(error);
    }
  }, [dispatch, fail, refreshThreads, threadId]);

  const renameSpeaker = useCallback(
    async (speaker: string, label: string) => {
      if (!threadId) return;
      try {
        const detail = await api.threads.renameSpeaker(threadId, speaker, label);
        dispatch({ type: "threadUpdated", detail });
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail, threadId],
  );

  const updateSegmentText = useCallback(
    async (segmentIndex: number, text: string) => {
      if (!threadId) return;
      try {
        const detail = await api.threads.updateSegmentText(threadId, segmentIndex, text);
        dispatch({ type: "threadUpdated", detail });
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail, threadId],
  );

  const copyTranscript = useCallback(async () => {
    if (!selected) return;
    try {
      await api.system.copyText(
        transcriptToText(selected.segments, selected.speakerLabels, formatDuration),
      );
    } catch (error) {
      fail(error);
    }
  }, [fail, selected]);

  const exportMarkdown = useCallback(async () => {
    if (!threadId) return;
    try {
      const path = await api.threads.exportMarkdown(threadId);
      await api.system.revealInFinder(path);
    } catch (error) {
      fail(error);
    }
  }, [fail, threadId]);

  const revealPath = useCallback(
    async (path: string) => {
      try {
        await api.system.revealInFinder(path);
      } catch (error) {
        fail(error);
      }
    },
    [fail],
  );

  return {
    copyTranscript,
    deleteThread,
    exportMarkdown,
    renameSpeaker,
    renameThread,
    revealPath,
    updateSegmentText,
  };
}
