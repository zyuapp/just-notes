import { expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { agentAccessApi } from "../../api/agentAccess";
import type { AgentGuideStatusPayload } from "../../bindings/AgentGuideStatusPayload";
import { useAgentAccessController } from "./useAgentAccessController";

const paths = {
  codex: "/Users/me/.agents/skills/just-notes",
  claudeCode: "/Users/me/.claude/skills/just-notes",
};

function status(agent: "codex" | "claudeCode", state: "notInstalled" | "installed") {
  return { agent, state, path: paths[agent], detail: null } satisfies AgentGuideStatusPayload;
}

function deferred<T>() {
  let resolve = (_value: T) => undefined;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function withController(
  run: (getController: () => ReturnType<typeof useAgentAccessController>) => Promise<void>,
) {
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
  let controller: ReturnType<typeof useAgentAccessController> | undefined;
  function Harness() {
    controller = useAgentAccessController();
    return null;
  }

  const root = createRoot(document.createElement("div"));
  try {
    await act(async () => {
      root.render(createElement(Harness));
      await Promise.resolve();
    });
    await run(() => {
      if (!controller) throw new Error("Agent Access controller was not rendered");
      return controller;
    });
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
}

test("controller confirms exact install paths and final shared-memory removal", async () => {
  await withController(async (getController) => {
    const initial = [status("codex", "notInstalled"), status("claudeCode", "notInstalled")];
    const installed = [status("codex", "installed"), status("claudeCode", "installed")];
    const claudeOnly = [status("codex", "notInstalled"), status("claudeCode", "installed")];
    const getStatuses = mock(async () => initial);
    const install = mock(async () => installed);
    const remove = mock(async (agent: "codex" | "claudeCode") =>
      agent === "codex" ? claudeOnly : initial);
    Object.assign(agentAccessApi, { getStatuses, install, remove });

    await act(async () => getController().refresh());
    expect(getController().viewState.statuses).toEqual(initial);

    act(() => getController().requestInstall(["codex", "claudeCode"]));
    expect(getController().viewState.confirmation).toEqual({
      kind: "install",
      agents: ["codex", "claudeCode"],
      destinations: [
        { agent: "codex", path: paths.codex },
        { agent: "claudeCode", path: paths.claudeCode },
      ],
    });
    await act(async () => getController().confirm());
    expect(install).toHaveBeenCalledWith(["codex", "claudeCode"]);
    expect(getController().viewState.statuses).toEqual(installed);

    await act(async () => getController().requestRemove("codex"));
    expect(remove).toHaveBeenLastCalledWith("codex", false);
    await act(async () => getController().requestRemove("claudeCode"));
    expect(getController().viewState.confirmation).toEqual({
      kind: "removeFinal",
      agent: "claudeCode",
    });
    await act(async () => getController().confirm());
    expect(remove).toHaveBeenLastCalledWith("claudeCode", true);
  });
});

test("older refreshes cannot replace install or remove results", async () => {
  await withController(async (getController) => {
    const initial = [status("codex", "notInstalled"), status("claudeCode", "notInstalled")];
    const installed = [status("codex", "installed"), status("claudeCode", "installed")];
    const claudeOnly = [status("codex", "notInstalled"), status("claudeCode", "installed")];
    const installRefresh = deferred<AgentGuideStatusPayload[]>();
    const removeRefresh = deferred<AgentGuideStatusPayload[]>();
    const getStatuses = mock()
      .mockResolvedValueOnce(initial)
      .mockImplementationOnce(() => installRefresh.promise)
      .mockImplementationOnce(() => removeRefresh.promise);
    Object.assign(agentAccessApi, {
      getStatuses,
      install: mock(async () => installed),
      remove: mock(async () => claudeOnly),
    });

    await act(async () => getController().refresh());
    let staleRefresh: Promise<void>;
    await act(async () => {
      staleRefresh = getController().refresh();
      await Promise.resolve();
    });
    act(() => getController().requestInstall(["codex", "claudeCode"]));
    await act(async () => getController().confirm());
    await act(async () => {
      installRefresh.resolve(initial);
      await staleRefresh!;
    });
    expect(getController().viewState.statuses).toEqual(installed);

    await act(async () => {
      staleRefresh = getController().refresh();
      await Promise.resolve();
    });
    await act(async () => getController().requestRemove("codex"));
    await act(async () => {
      removeRefresh.resolve(installed);
      await staleRefresh!;
    });
    expect(getController().viewState.statuses).toEqual(claudeOnly);
  });
});
