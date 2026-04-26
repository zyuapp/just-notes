import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { invokeCommand } from "./transport";

export const transcriptionApi = {
  getStatus(): Promise<TranscriptionStatusPayload> {
    return invokeCommand("get_transcription_status");
  },
};
