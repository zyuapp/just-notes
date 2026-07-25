import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import type { StorageUsagePayload } from "../bindings/StorageUsagePayload";
import type { PrivacyPane } from "../lib/permissionStatus";
import { invokeCommand } from "./transport";

export const systemApi = {
  getPermissions(): Promise<PermissionsPayload> {
    return invokeCommand("get_permissions_status");
  },

  getStorageUsage(): Promise<StorageUsagePayload> {
    return invokeCommand("get_storage_usage");
  },

  deleteReclaimableRawAudio(): Promise<StorageUsagePayload> {
    return invokeCommand("delete_reclaimable_raw_audio");
  },

  revealInFinder(path: string): Promise<void> {
    return invokeCommand("reveal_in_finder", { path });
  },

  copyText(text: string): Promise<void> {
    return invokeCommand("copy_text_to_clipboard", { text });
  },

  openPrivacySettings(
    pane: PrivacyPane,
  ): Promise<void> {
    return invokeCommand("open_privacy_settings", { pane });
  },

  openExternalUrl(url: string): Promise<void> {
    return invokeCommand("open_external_url", { url });
  },

  openLegalDocument(document: "privacy" | "notices"): Promise<void> {
    return invokeCommand("open_legal_document", { document });
  },
};
