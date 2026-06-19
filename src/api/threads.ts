import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { invokeCommand } from "./transport";

export const threadsApi = {
  list(): Promise<ThreadSummary[]> {
    return invokeCommand("list_threads");
  },

  create(): Promise<ThreadDetail> {
    return invokeCommand("create_thread");
  },

  get(threadId: string): Promise<ThreadDetail> {
    return invokeCommand("get_thread", { threadId });
  },

  rename(threadId: string, title: string): Promise<ThreadDetail> {
    return invokeCommand("rename_thread", { threadId, title });
  },

  listArchived(): Promise<ThreadSummary[]> {
    return invokeCommand("list_archived_threads");
  },

  archive(threadId: string): Promise<void> {
    return invokeCommand("archive_thread", { threadId });
  },

  restore(threadId: string): Promise<void> {
    return invokeCommand("restore_thread", { threadId });
  },

  delete(threadId: string): Promise<void> {
    return invokeCommand("delete_thread", { threadId });
  },

  renameSpeaker(threadId: string, speaker: string, label: string): Promise<ThreadDetail> {
    return invokeCommand("rename_speaker", { threadId, speaker, label });
  },

  updateSegmentText(threadId: string, segmentIndex: number, text: string): Promise<ThreadDetail> {
    return invokeCommand("update_segment_text", { threadId, segmentIndex, text });
  },

  search(query: string): Promise<ThreadSummary[]> {
    return invokeCommand("search_threads", { query });
  },

  exportMarkdown(threadId: string): Promise<string> {
    return invokeCommand("export_thread_markdown", { threadId });
  },
};
