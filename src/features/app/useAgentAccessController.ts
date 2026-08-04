import { useCallback, useRef, useState } from "react";
import { agentAccessApi } from "../../api/agentAccess";
import { getApiErrorMessage } from "../../api/errors";
import type { AgentGuideStatusPayload } from "../../bindings/AgentGuideStatusPayload";
import type { AgentId } from "../../bindings/AgentId";

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

export function useAgentAccessController() {
  const [statuses, setStatuses] = useState<AgentGuideStatusPayload[] | null>(null);
  const statusesRef = useRef(statuses);
  const statusRevisionRef = useRef(0);
  const [busy, setBusy] = useState<AgentAccessViewState["busy"]>(null);
  const [confirmation, setConfirmation] = useState<AgentAccessConfirmation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const store = useCallback((statuses: AgentGuideStatusPayload[]) => {
    statusesRef.current = statuses;
    setStatuses(statuses);
  }, []);

  const refresh = useCallback(async () => {
    const revision = ++statusRevisionRef.current;
    setBusy((current) => current ?? "loading");
    try {
      const loaded = await agentAccessApi.getStatuses();
      if (revision === statusRevisionRef.current) {
        store(loaded);
        setError(null);
      }
    } catch (cause) {
      if (revision === statusRevisionRef.current) setError(getApiErrorMessage(cause));
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
    const revision = ++statusRevisionRef.current;
    setBusy(agent);
    setError(null);
    try {
      const loaded = await agentAccessApi.remove(agent, removeSharedMemory);
      if (revision === statusRevisionRef.current) store(loaded);
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
    const revision = ++statusRevisionRef.current;
    try {
      const loaded = await agentAccessApi.install(request.agents);
      if (revision === statusRevisionRef.current) store(loaded);
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
    viewState: { statuses, busy, confirmation, error },
    cancelConfirmation: () => setConfirmation(null),
    confirm,
    refresh,
    requestInstall,
    requestRemove,
    reveal,
  };
}
