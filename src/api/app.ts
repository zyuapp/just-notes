import type { AppInfo } from "../bindings/AppInfo";
import { invokeCommand } from "./transport";

export const appApi = {
  getInfo(): Promise<AppInfo> {
    return invokeCommand("get_app_info");
  },
};
