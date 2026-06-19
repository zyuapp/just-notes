import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";

export function downloadPercent(model: TranscriptionModelStatus): number {
  if (model.totalBytes === 0) return 0;
  return Math.min(100, Math.floor((model.progressBytes * 100) / model.totalBytes));
}

export function downloadActionLabel(): string {
  return "Download Parakeet";
}

export function isModelDownloadActive(model: TranscriptionModelStatus): boolean {
  return model.downloadState === "downloading" || model.downloadState === "installing";
}
