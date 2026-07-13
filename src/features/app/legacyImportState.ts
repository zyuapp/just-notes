import type { LegacyImportPreview } from "../../bindings/LegacyImportPreview";
import type { LegacyImportResult } from "../../bindings/LegacyImportResult";

export type LegacyImportViewState =
  | { stage: "intro"; error?: string }
  | { stage: "locating" }
  | { stage: "preview"; preview: LegacyImportPreview }
  | { stage: "importing"; preview: LegacyImportPreview }
  | { stage: "complete"; result: LegacyImportResult };

export function openLegacyImportState(
  current: LegacyImportViewState | null,
): LegacyImportViewState {
  return current ?? { stage: "intro" };
}
