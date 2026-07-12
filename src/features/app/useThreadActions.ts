import { useCallback } from "react";
import { api, getApiErrorMessage } from "../../api";
import { formatDuration } from "../../lib/format";
import { findThreadSummary, neighborThreadId } from "../../lib/threads";
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

  const archiveThread = useCallback(
    async (targetId: string) => {
      const summary = findThreadSummary(state.threads, state.selectedThread, targetId);
      const wasSelected = state.selectedThreadId === targetId;
      try {
        await api.threads.archive(targetId);
        // Optimistically drop the row and bounce the archive icon when we know it.
        if (summary) {
          dispatch({ type: "threadArchived", summary });
        }
        // Reconcile from the backend: the open thread needs a neighbor selected,
        // and a thread we couldn't drop optimistically needs a refetch.
        if (wasSelected) {
          await refreshThreads(neighborThreadId(state.threads, targetId));
        } else if (!summary) {
          await refreshThreads();
        }
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail, refreshThreads, state.threads, state.selectedThread, state.selectedThreadId],
  );

  const restoreThread = useCallback(
    async (targetId: string) => {
      try {
        await api.threads.restore(targetId);
        await refreshThreads();
      } catch (error) {
        fail(error);
      }
    },
    [fail, refreshThreads],
  );

  const deleteArchivedThread = useCallback(
    async (targetId: string) => {
      try {
        await api.threads.delete(targetId);
      } catch (error) {
        fail(error);
      }
    },
    [fail],
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
      await api.system.copyText(transcriptToText(selected.segments, formatDuration));
    } catch (error) {
      fail(error);
    }
  }, [fail, selected]);

  const exportMarkdown = useCallback(
    async (targetId: string) => {
      try {
        const path = await api.threads.exportMarkdown(targetId);
        await api.system.revealInFinder(path);
      } catch (error) {
        fail(error);
      }
    },
    [fail],
  );

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
    archiveThread,
    copyTranscript,
    deleteArchivedThread,
    exportMarkdown,
    renameThread,
    restoreThread,
    revealPath,
    updateSegmentText,
  };
}
