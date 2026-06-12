import { invokeCommand } from "./transport";

export const indicatorApi = {
  setVisibleWidth(visibleWidth: number): Promise<void> {
    return invokeCommand("set_indicator_width", { visibleWidth });
  },

  openMainWindow(): Promise<void> {
    return invokeCommand("open_main_window");
  },
};
