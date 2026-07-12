import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";

export function downloadPercent(model: TranscriptionModelStatus): number {
  if (model.totalBytes === 0) return 0;
  return Math.min(100, Math.floor((model.progressBytes * 100) / model.totalBytes));
}

export function downloadActionLabel(model?: TranscriptionModelStatus): string {
  const size = model?.displaySize.trim();
  return size ? `Download Parakeet · ${size}` : "Download Parakeet";
}

export function downloadConfirmationMessage(model: TranscriptionModelStatus): string {
  const size = model.displaySize.trim() || "about 460 MB";
  return `Local transcription requires a one-time ${size} download. Audio stays on this Mac.`;
}

export function isModelDownloadActive(model: TranscriptionModelStatus): boolean {
  return model.downloadState === "downloading" || model.downloadState === "installing";
}
