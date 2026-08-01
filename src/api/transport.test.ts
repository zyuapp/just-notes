import { beforeEach, describe, expect, mock, test } from "bun:test";
import { ApiError } from "./errors";

const invokeMock = mock();

mock.module("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

const { invokeCommand } = await import("./transport");
const { agentAccessApi } = await import("./agentAccess");
const { meetingsApi } = await import("./meetings");

describe("invokeCommand", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  test("passes command names and arguments to Tauri invoke", async () => {
    const payload = [{ id: "thread-1" }];
    invokeMock.mockResolvedValueOnce(payload);

    const result = await invokeCommand("list_threads", { limit: 1 });

    expect(result).toEqual(payload);
    expect(invokeMock.mock.calls).toEqual([["list_threads", { limit: 1 }]]);
  });

  test("normalizes invoke failures", async () => {
    invokeMock.mockRejectedValueOnce("Tauri rejected command");

    try {
      await invokeCommand("stop_recording");
      throw new Error("Expected invokeCommand to reject");
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      expect((error as ApiError).message).toBe("Tauri rejected command");
    }
  });
});

describe("meetingsApi", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  test("requests Calendar access with the Calendar-only command", async () => {
    invokeMock.mockResolvedValueOnce({});

    await meetingsApi.requestCalendarAccess();

    expect(invokeMock.mock.calls).toEqual([["request_meeting_calendar_access", undefined]]);
  });

  test("requests notification access with the notification-only command", async () => {
    invokeMock.mockResolvedValueOnce({});

    await meetingsApi.requestNotificationAccess();

    expect(invokeMock.mock.calls).toEqual([["request_meeting_notification_access", undefined]]);
  });
});

describe("agentAccessApi", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue([]);
  });

  test("uses typed agent identifiers and backend-selected reveal paths", async () => {
    await agentAccessApi.install(["codex", "claudeCode"]);
    await agentAccessApi.remove("codex", true);
    await agentAccessApi.reveal("claudeCode");

    expect(invokeMock.mock.calls).toEqual([
      ["install_agent_guides", { agents: ["codex", "claudeCode"] }],
      ["remove_agent_guide", { agent: "codex", removeSharedMemory: true }],
      ["reveal_agent_guide", { agent: "claudeCode" }],
    ]);
  });
});
