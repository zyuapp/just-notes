import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { invokeCommand } from "./transport";

export const transcriptionApi = {
  getStatus(): Promise<TranscriptionStatusPayload> {
    return invokeCommand("get_transcription_status");
  },

  startModelDownload(): Promise<TranscriptionStatusPayload> {
    return invokeCommand("start_transcription_model_download");
  },

  cancelModelDownload(): Promise<TranscriptionStatusPayload> {
    return invokeCommand("cancel_transcription_model_download");
  },

  deleteModel(): Promise<TranscriptionStatusPayload> {
    return invokeCommand("delete_transcription_model");
  },
};
