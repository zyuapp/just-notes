import type { RecordingPayload } from "../bindings/RecordingPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import { invokeCommand } from "./transport";

export const recordingApi = {
  start(threadId: string | null): Promise<RecordingPayload> {
    return invokeCommand("start_recording", { threadId });
  },

  startFixture(threadId: string | null): Promise<RecordingPayload> {
    return invokeCommand("start_fixture_recording", { threadId });
  },

  stop(): Promise<ThreadDetail> {
    return invokeCommand("stop_recording");
  },
};
