import { expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement, useReducer } from "react";
import { createRoot } from "react-dom/client";
import { agentAccessApi } from "../../api/agentAccess";
import type { AgentGuideStatusPayload } from "../../bindings/AgentGuideStatusPayload";
import { appReducer, initialAppState } from "./state";
import { useAgentAccessController } from "./useAgentAccessController";

const paths = {
  codex: "/Users/me/.agents/skills/just-notes",
  claudeCode: "/Users/me/.claude/skills/just-notes",
};

function status(agent: "codex" | "claudeCode", state: "notInstalled" | "installed") {
  return { agent, state, path: paths[agent], detail: null } satisfies AgentGuideStatusPayload;
}

test("controller confirms exact install paths and final shared-memory removal", async () => {
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
  const originals = { ...agentAccessApi };
  const initial = [status("codex", "notInstalled"), status("claudeCode", "notInstalled")];
  const installed = [status("codex", "installed"), status("claudeCode", "installed")];
  const claudeOnly = [status("codex", "notInstalled"), status("claudeCode", "installed")];
  const getStatuses = mock(async () => initial);
  const install = mock(async () => installed);
  const remove = mock(async (agent: "codex" | "claudeCode") =>
    agent === "codex" ? claudeOnly : initial);
  Object.assign(agentAccessApi, { getStatuses, install, remove });

  let controller: ReturnType<typeof useAgentAccessController> | undefined;
  function Harness() {
    const [state, dispatch] = useReducer(appReducer, { ...initialAppState, settingsOpen: true });
    controller = useAgentAccessController(state, dispatch);
    return null;
  }

  const root = createRoot(document.createElement("div"));
  try {
    await act(async () => {
      root.render(createElement(Harness));
      await Promise.resolve();
    });
    await act(async () => controller?.refresh());
    expect(controller?.viewState.statuses).toEqual(initial);

    act(() => controller?.requestInstall(["codex", "claudeCode"]));
    expect(controller?.viewState.confirmation).toEqual({
      kind: "install",
      agents: ["codex", "claudeCode"],
      destinations: [
        { agent: "codex", path: paths.codex },
        { agent: "claudeCode", path: paths.claudeCode },
      ],
    });
    await act(async () => controller?.confirm());
    expect(install).toHaveBeenCalledWith(["codex", "claudeCode"]);
    expect(controller?.viewState.statuses).toEqual(installed);

    await act(async () => controller?.requestRemove("codex"));
    expect(remove).toHaveBeenLastCalledWith("codex", false);
    await act(async () => controller?.requestRemove("claudeCode"));
    expect(controller?.viewState.confirmation).toEqual({ kind: "removeFinal", agent: "claudeCode" });
    await act(async () => controller?.confirm());
    expect(remove).toHaveBeenLastCalledWith("claudeCode", true);
  } finally {
    await act(async () => root.unmount());
    browserWindow.close();
    Object.assign(agentAccessApi, originals);
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      navigator: previous.navigator,
      IS_REACT_ACT_ENVIRONMENT: previous.act,
    });
  }
});
