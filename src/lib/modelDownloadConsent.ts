import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { downloadConfirmationMessage } from "./transcriptionModel";

export async function requestModelDownload(
  model: TranscriptionModelStatus | undefined,
  confirm: (message: string) => boolean,
  start: () => Promise<TranscriptionStatusPayload>,
): Promise<TranscriptionStatusPayload | null> {
  if (model && !confirm(downloadConfirmationMessage(model))) return null;
  return start();
}
