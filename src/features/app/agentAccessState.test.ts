import { expect, test } from "bun:test";
import type { AgentGuideStatusPayload } from "../../bindings/AgentGuideStatusPayload";
import { appReducer, initialAppState } from "./state";

test("the reducer stores filesystem-derived agent guide status", () => {
  const statuses: AgentGuideStatusPayload[] = [{
    agent: "codex",
    state: "installed",
    path: "/Users/me/.agents/skills/just-notes",
    detail: null,
  }];
  const updated = appReducer(initialAppState, {
    type: "agentGuideStatusesLoaded",
    statuses,
  });

  expect(updated.agentGuideStatuses).toBe(statuses);
});
