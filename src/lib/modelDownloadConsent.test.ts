import { expect, mock, test } from "bun:test";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { requestModelDownload } from "./modelDownloadConsent";

const model = {
  id: "parakeet",
  name: "Parakeet",
  displaySize: "460 MB",
} as TranscriptionModelStatus;
const status = {} as TranscriptionStatusPayload;

test("declining model consent does not start a download", async () => {
  const start = mock(async () => status);

  expect(await requestModelDownload(model, () => false, start)).toBeNull();
  expect(start).toHaveBeenCalledTimes(0);
});

test("accepting model consent starts exactly once with a size-aware prompt", async () => {
  const confirm = mock(() => true);
  const start = mock(async () => status);

  expect(await requestModelDownload(model, confirm, start)).toBe(status);
  expect(confirm).toHaveBeenCalledTimes(1);
  expect(confirm.mock.calls[0]?.[0]).toContain("460 MB");
  expect(start).toHaveBeenCalledTimes(1);
});
