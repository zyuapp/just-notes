import { useCallback, useRef, useState } from "react";
import { agentAccessApi } from "../../api/agentAccess";
import { getApiErrorMessage } from "../../api/errors";
import type { AgentGuideStatusPayload } from "../../bindings/AgentGuideStatusPayload";
import type { AgentId } from "../../bindings/AgentId";
import type { AppAction, AppState } from "./state";

type AppDispatch = (action: AppAction) => void;
type Destination = { agent: AgentId; path: string };

export type AgentAccessConfirmation =
  | { kind: "install"; agents: AgentId[]; destinations: Destination[] }
  | { kind: "removeFinal"; agent: AgentId };

export type AgentAccessViewState = {
  statuses: AgentGuideStatusPayload[] | null;
  busy: AgentId | "both" | "loading" | null;
  confirmation: AgentAccessConfirmation | null;
  error: string | null;
};

export function useAgentAccessController(state: AppState, dispatch: AppDispatch) {
  const statusesRef = useRef(state.agentGuideStatuses);
  const [busy, setBusy] = useState<AgentAccessViewState["busy"]>(null);
  const [confirmation, setConfirmation] = useState<AgentAccessConfirmation | null>(null);
  const [error, setError] = useState<string | null>(null);
  statusesRef.current = state.agentGuideStatuses;

  const store = useCallback((statuses: AgentGuideStatusPayload[]) => {
    statusesRef.current = statuses;
    dispatch({ type: "agentGuideStatusesLoaded", statuses });
  }, [dispatch]);

  const refresh = useCallback(async () => {
    setBusy((current) => current ?? "loading");
    try {
      store(await agentAccessApi.getStatuses());
      setError(null);
    } catch (cause) {
      setError(getApiErrorMessage(cause));
    } finally {
      setBusy((current) => current === "loading" ? null : current);
    }
  }, [store]);

  const requestInstall = useCallback((agents: AgentId[]) => {
    const statuses = statusesRef.current;
    if (!statuses) return;
    const destinations = agents.flatMap((agent) => {
      const status = statuses.find((item) => item.agent === agent);
      return status ? [{ agent, path: status.path }] : [];
    });
    setConfirmation({ kind: "install", agents, destinations });
  }, []);

  const runRemove = useCallback(async (agent: AgentId, removeSharedMemory: boolean) => {
    setBusy(agent);
    setError(null);
    try {
      store(await agentAccessApi.remove(agent, removeSharedMemory));
    } catch (cause) {
      setError(getApiErrorMessage(cause));
    } finally {
      setBusy(null);
    }
  }, [store]);

  const requestRemove = useCallback(async (agent: AgentId) => {
    const installed = statusesRef.current?.filter((status) => status.state === "installed") ?? [];
    if (installed.length === 1 && installed[0].agent === agent) {
      setConfirmation({ kind: "removeFinal", agent });
    } else {
      await runRemove(agent, false);
    }
  }, [runRemove]);

  const confirm = useCallback(async () => {
    const request = confirmation;
    if (!request) return;
    setConfirmation(null);
    if (request.kind === "removeFinal") {
      await runRemove(request.agent, true);
      return;
    }
    setBusy(request.agents.length > 1 ? "both" : request.agents[0]);
    setError(null);
    try {
      store(await agentAccessApi.install(request.agents));
    } catch (cause) {
      setError(getApiErrorMessage(cause));
    } finally {
      setBusy(null);
    }
  }, [confirmation, runRemove, store]);

  const reveal = useCallback(async (agent: AgentId) => {
    try {
      await agentAccessApi.reveal(agent);
    } catch (cause) {
      setError(getApiErrorMessage(cause));
    }
  }, []);

  return {
    viewState: { statuses: state.agentGuideStatuses, busy, confirmation, error },
    cancelConfirmation: () => setConfirmation(null),
    confirm,
    refresh,
    requestInstall,
    requestRemove,
    reveal,
  };
}
