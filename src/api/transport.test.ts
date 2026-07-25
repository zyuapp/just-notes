import { beforeEach, describe, expect, mock, test } from "bun:test";
import { ApiError } from "./errors";

const invokeMock = mock();

mock.module("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

const { invokeCommand } = await import("./transport");
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
