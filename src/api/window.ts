import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "./events";
import { toApiError } from "./errors";

export const windowApi = {
  async onFocusChanged(handler: (focused: boolean) => void): Promise<UnlistenFn> {
    try {
      return await getCurrentWindow().onFocusChanged(({ payload }) => handler(payload));
    } catch (error) {
      throw toApiError(error);
    }
  },
};
