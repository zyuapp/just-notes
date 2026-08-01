import { expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { AgentAccessSettingsSection } from "./AgentAccessSettingsSection";

test("announces Agent Access progress and failures", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    navigator: globalThis.navigator,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT,
  };
  Object.assign(globalThis, {
    window: browserWindow,
    document: browserWindow.document,
    navigator: browserWindow.navigator,
    IS_REACT_ACT_ENVIRONMENT: true,
  });
  const container = document.createElement("div");
  const root = createRoot(container);
  const refresh = mock();
  const baseActions = {
    onInstall: () => undefined,
    onRemove: () => undefined,
    onReveal: () => undefined,
    onRefresh: refresh,
    onCancelConfirmation: () => undefined,
    onConfirm: () => undefined,
  };
  try {
    await act(async () => root.render(createElement(AgentAccessSettingsSection, {
      viewState: {
        statuses: [],
        busy: "both",
        confirmation: null,
        error: "Installation failed",
      },
      ...baseActions,
    })));

    expect(container.querySelector('[role="status"]')?.textContent).toContain("Updating both guides");
    expect(container.querySelector('[role="status"] svg')?.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector('[role="alert"]')?.textContent).toBe("Installation failed");

    await act(async () => root.render(createElement(AgentAccessSettingsSection, {
      viewState: { statuses: null, busy: null, confirmation: null, error: "Status read failed" },
      ...baseActions,
    })));
    expect(container.querySelector('[role="status"]')).toBeNull();
    const retry = container.querySelector(".agent-access-retry button") as HTMLButtonElement;
    retry.click();
    expect(refresh).toHaveBeenCalledTimes(1);

    await act(async () => root.render(createElement(AgentAccessSettingsSection, {
      viewState: {
        statuses: [{
          agent: "codex",
          state: "notInstalled",
          path: "/Users/me/.agents/skills/just-notes",
          detail: "Installation failed: permission denied",
        }],
        busy: null,
        confirmation: null,
        error: null,
      },
      ...baseActions,
    })));
    expect(container.querySelector('[role="alert"]')?.textContent).toContain("permission denied");
  } finally {
    await act(async () => root.unmount());
    browserWindow.close();
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      navigator: previous.navigator,
      IS_REACT_ACT_ENVIRONMENT: previous.act,
    });
  }
});
