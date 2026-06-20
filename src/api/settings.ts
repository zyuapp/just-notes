import type { AppSettings } from "../bindings/AppSettings";
import { invokeCommand } from "./transport";

export const settingsApi = {
  get(): Promise<AppSettings> {
    return invokeCommand("get_settings");
  },

  update(settings: AppSettings): Promise<AppSettings> {
    return invokeCommand("update_settings", { settings });
  },
};
