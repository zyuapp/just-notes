import { describe, expect, test } from "bun:test";
import { ApiError, getApiErrorMessage, toApiError } from "./errors";

describe("toApiError", () => {
  test("preserves existing ApiError instances", () => {
    const original = new ApiError("Already normalized");

    expect(toApiError(original)).toBe(original);
  });

  test("uses Error messages and keeps the original cause", () => {
    const original = new Error("Native failure");
    const normalized = toApiError(original);

    expect(normalized).toBeInstanceOf(ApiError);
    expect(normalized.message).toBe("Native failure");
    expect(normalized.cause).toBe(original);
  });

  test("uses string errors as the message and cause", () => {
    const normalized = toApiError("IPC failed");

    expect(normalized.message).toBe("IPC failed");
    expect(normalized.cause).toBe("IPC failed");
  });

  test("falls back for unknown error shapes", () => {
    const cause = { code: "unknown" };
    const normalized = toApiError(cause);

    expect(normalized.message).toBe("An unexpected app error occurred.");
    expect(normalized.cause).toBe(cause);
  });
});

describe("getApiErrorMessage", () => {
  test("returns the normalized message", () => {
    expect(getApiErrorMessage("Readable failure")).toBe("Readable failure");
  });
});
