import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { TranscriptionProvider } from "../bindings/TranscriptionProvider";
import { invokeCommand } from "./transport";

export const transcriptionApi = {
  getStatus(): Promise<TranscriptionStatusPayload> {
    return invokeCommand("get_transcription_status");
  },

  startModelDownload(provider: TranscriptionProvider): Promise<TranscriptionStatusPayload> {
    return invokeCommand("start_transcription_model_download", { provider });
  },

  cancelModelDownload(provider: TranscriptionProvider): Promise<TranscriptionStatusPayload> {
    return invokeCommand("cancel_transcription_model_download", { provider });
  },
};
