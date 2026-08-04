import { expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { AgentAccessConfirmationDialog } from "./AgentAccessConfirmationDialog";

test("shows install destinations and the final-memory warning before confirmation", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    navigator: globalThis.navigator,
    HTMLElement: globalThis.HTMLElement,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT,
  };
  Object.assign(globalThis, {
    window: browserWindow,
    document: browserWindow.document,
    navigator: browserWindow.navigator,
    HTMLElement: browserWindow.HTMLElement,
    IS_REACT_ACT_ENVIRONMENT: true,
  });
  const container = document.createElement("div");
  const root = createRoot(container);
  const cancel = mock();
  const confirm = mock();
  try {
    await act(async () => root.render(createElement(AgentAccessConfirmationDialog, {
      confirmation: {
        kind: "install",
        agents: ["codex", "claudeCode"],
        destinations: [
          { agent: "codex", path: "/codex/just-notes" },
          { agent: "claudeCode", path: "/claude/just-notes" },
        ],
      },
      onCancel: cancel,
      onConfirm: confirm,
    })));

    const dialog = container.querySelector('[role="alertdialog"]') as HTMLElement;
    expect(dialog.textContent).toContain("An external agent may process transcript text");
    expect([...dialog.querySelectorAll("code")].map((node) => node.textContent)).toEqual([
      "/codex/just-notes",
      "/claude/just-notes",
    ]);
    const installButtons = [...dialog.querySelectorAll("button")];
    installButtons.find((button) => button.textContent === "Cancel")?.click();
    installButtons.find((button) => button.textContent === "Install")?.click();
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(confirm).toHaveBeenCalledTimes(1);

    await act(async () => root.render(createElement(AgentAccessConfirmationDialog, {
      confirmation: { kind: "removeFinal", agent: "codex" },
      onCancel: cancel,
      onConfirm: confirm,
    })));
    const removal = container.querySelector('[role="alertdialog"]') as HTMLElement;
    expect(removal.textContent).toContain("permanently deletes shared Agent Access memory");
    removal.querySelector<HTMLButtonElement>("button:last-child")?.click();
    expect(confirm).toHaveBeenCalledTimes(2);
  } finally {
    await act(async () => root.unmount());
    browserWindow.close();
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      navigator: previous.navigator,
      HTMLElement: previous.HTMLElement,
      IS_REACT_ACT_ENVIRONMENT: previous.act,
    });
  }
});
