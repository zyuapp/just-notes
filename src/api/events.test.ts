import { beforeEach, describe, expect, mock, test } from "bun:test";
import { ApiError } from "./errors";

const listenMock = mock();

mock.module("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

const { eventsApi } = await import("./events");

describe("eventsApi", () => {
  beforeEach(() => {
    listenMock.mockReset();
  });

  test("registers meter events and unwraps event payloads", async () => {
    const dispose = mock();
    const handler = mock();
    const payload = {
      threadId: "thread-1",
      micLevel: 0.2,
      systemLevel: 0.8,
      elapsedMs: 1200,
    };

    listenMock.mockImplementationOnce(async () => dispose);

    const unlisten = await eventsApi.onMeter(handler);
    const [eventName, registeredHandler] = listenMock.mock.calls[0];
    registeredHandler({ payload });

    expect(eventName).toBe("meter-update");
    expect(unlisten).toBe(dispose);
    expect(handler.mock.calls).toEqual([[payload]]);
  });

  test("normalizes listen failures", async () => {
    listenMock.mockRejectedValueOnce("Listener rejected");

    try {
      await eventsApi.onMeter(mock());
      throw new Error("Expected onMeter to reject");
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      expect((error as ApiError).message).toBe("Listener rejected");
    }
  });

  test("registers the native File menu import request", async () => {
    const dispose = mock();
    const handler = mock();
    listenMock.mockImplementationOnce(async () => dispose);

    await eventsApi.onLegacyImportRequested(handler);
    const [eventName, registeredHandler] = listenMock.mock.calls[0];
    registeredHandler({ payload: undefined });

    expect(eventName).toBe("legacy-import-requested");
    expect(handler).toHaveBeenCalledTimes(1);
  });
});
