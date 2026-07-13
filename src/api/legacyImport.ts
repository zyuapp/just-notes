import type { LegacyImportPreview } from "../bindings/LegacyImportPreview";
import type { LegacyImportResult } from "../bindings/LegacyImportResult";
import { invokeCommand } from "./transport";

export const legacyImportApi = {
  prepare(): Promise<LegacyImportPreview | null> {
    return invokeCommand("prepare_legacy_import");
  },

  confirm(): Promise<LegacyImportResult> {
    return invokeCommand("confirm_legacy_import");
  },

  cancel(): Promise<void> {
    return invokeCommand("cancel_legacy_import");
  },
};
