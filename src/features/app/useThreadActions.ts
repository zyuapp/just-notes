import { useCallback } from "react";
import { api, getApiErrorMessage } from "../../api";
import { formatDuration } from "../../lib/format";
import { neighborThreadId } from "../../lib/threads";
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
      try {
        const summary =
          state.threads.find((thread) => thread.id === targetId) ??
          (state.selectedThread?.summary.id === targetId ? state.selectedThread.summary : null);
        const wasSelected = state.selectedThreadId === targetId;
        const neighborId = wasSelected ? neighborThreadId(state.threads, targetId) : undefined;
        await api.threads.archive(targetId);
        if (summary) {
          dispatch({ type: "threadArchived", summary });
        }
        // Open thread needs a neighbor selected; a missing summary needs a refresh.
        if (wasSelected) {
          await refreshThreads(neighborId);
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
      // Retract the undo affordance up front so it can't race the restore.
      dispatch({ type: "archiveNoticeCleared", threadId: targetId });
      try {
        await api.threads.restore(targetId);
        await refreshThreads();
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail, refreshThreads],
  );

  const deleteArchivedThread = useCallback(
    async (targetId: string) => {
      // A deleted thread can't be restored, so retract the undo before deleting.
      dispatch({ type: "archiveNoticeCleared", threadId: targetId });
      try {
        await api.threads.delete(targetId);
      } catch (error) {
        fail(error);
      }
    },
    [dispatch, fail],
  );

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
    renameSpeaker,
    renameThread,
    restoreThread,
    revealPath,
    updateSegmentText,
  };
}
