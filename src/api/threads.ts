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
};
