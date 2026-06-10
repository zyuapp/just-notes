import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import { invokeCommand } from "./transport";

export const systemApi = {
  getPermissions(): Promise<PermissionsPayload> {
    return invokeCommand("get_permissions_status");
  },

  revealInFinder(path: string): Promise<void> {
    return invokeCommand("reveal_in_finder", { path });
  },

  copyText(text: string): Promise<void> {
    return invokeCommand("copy_text_to_clipboard", { text });
  },

  openPrivacySettings(pane: "microphone" | "system-audio"): Promise<void> {
    return invokeCommand("open_privacy_settings", { pane });
  },
};
