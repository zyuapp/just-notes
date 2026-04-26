import { beforeEach, describe, expect, mock, test } from "bun:test";
import { ApiError } from "./errors";

const invokeMock = mock();

mock.module("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

const { invokeCommand } = await import("./transport");

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
