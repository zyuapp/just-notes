import { invokeCommand } from "./transport";

export const indicatorApi = {
  setVisibleWidth(visibleWidth: number): Promise<void> {
    return invokeCommand("set_indicator_width", { visibleWidth });
  },

  getState(): Promise<boolean> {
    return invokeCommand("get_indicator_state");
  },

  openMainWindow(): Promise<void> {
    return invokeCommand("open_main_window");
  },
};
