import { beforeEach, expect, mock, test } from "bun:test";
import { ApiError } from "./errors";

const onFocusChangedMock = mock();

mock.module("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onFocusChanged: onFocusChangedMock,
  }),
}));

const { windowApi } = await import("./window");

beforeEach(() => {
  onFocusChangedMock.mockReset();
});

test("window focus events expose the focused state", async () => {
  const unlisten = mock();
  const handler = mock();
  onFocusChangedMock.mockResolvedValueOnce(unlisten);

  const stopListening = await windowApi.onFocusChanged(handler);
  const [registeredHandler] = onFocusChangedMock.mock.calls[0];
  registeredHandler({ payload: true });

  expect(stopListening).toBe(unlisten);
  expect(handler.mock.calls).toEqual([[true]]);
});

test("window focus listener failures are normalized", async () => {
  onFocusChangedMock.mockRejectedValueOnce("Focus listener failed");

  try {
    await windowApi.onFocusChanged(mock());
    throw new Error("Expected onFocusChanged to reject");
  } catch (error) {
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).message).toBe("Focus listener failed");
  }
});
