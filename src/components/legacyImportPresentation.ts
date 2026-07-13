import type { LegacyImportViewState } from "../features/app/legacyImportState";

type CompleteResult = Extract<LegacyImportViewState, { stage: "complete" }>["result"];

export function importActionLabel(importing: boolean, count: number) {
  if (importing) return "Importing…";
  if (count === 0) return "Finish import";
  return `Import ${count} recording${count === 1 ? "" : "s"}`;
}

export function completeImportSummary(result: CompleteResult) {
  const parts = [`${result.imported} recording${result.imported === 1 ? "" : "s"} imported`];
  if (result.duplicates > 0) {
    parts.push(`${result.duplicates} duplicate${result.duplicates === 1 ? "" : "s"} skipped`);
  }
  if (result.conflicts > 0) {
    parts.push(`${result.conflicts} conflict${result.conflicts === 1 ? "" : "s"} preserved`);
  }
  return parts.join(" · ");
}

export function formatImportBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${Math.round(bytes / (1024 * 1024))} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}
