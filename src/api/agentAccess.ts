import type { AgentGuideStatusPayload } from "../bindings/AgentGuideStatusPayload";
import type { AgentId } from "../bindings/AgentId";
import { invokeCommand } from "./transport";

export const agentAccessApi = {
  getStatuses(): Promise<AgentGuideStatusPayload[]> {
    return invokeCommand("get_agent_guide_statuses");
  },

  install(agents: AgentId[]): Promise<AgentGuideStatusPayload[]> {
    return invokeCommand("install_agent_guides", { agents });
  },

  remove(agent: AgentId, removeSharedMemory: boolean): Promise<AgentGuideStatusPayload[]> {
    return invokeCommand("remove_agent_guide", { agent, removeSharedMemory });
  },

  reveal(agent: AgentId): Promise<void> {
    return invokeCommand("reveal_agent_guide", { agent });
  },
};
