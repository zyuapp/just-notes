import { expect, test } from "bun:test";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import { runRecordingStart } from "./recordingStart";
import type { AppAction } from "./state";

const payload = {
  thread: {
    summary: {
      id: "thread-1",
      title: "Design review",
      createdAtMs: 100,
      updatedAtMs: 100,
      segmentCount: 0,
      status: "recording",
      durationMs: 0,
      snippet: "",
      hasAudio: false,
      path: "/threads/thread-1",
    },
    segments: [],
    transcriptMarkdownPath: "/threads/thread-1/transcript.md",
  },
  transcription: {
    ready: true,
    engineExists: true,
    modelExists: true,
    enginePath: "",
    modelPath: "",
    modelName: "Parakeet",
    availableModels: [],
    message: "Ready",
  },
} satisfies RecordingPayload;

test("recording start dispatches success before refreshing threads", async () => {
  const actions: AppAction[] = [];
  const order: string[] = [];

  await runRecordingStart({
    dispatch: (action) => {
      actions.push(action);
      order.push(action.type);
    },
    start: async () => payload,
    refreshThreads: async (threadId) => {
      expect(threadId).toBe("thread-1");
      order.push("refreshThreads");
    },
  });

  expect(order).toEqual(["recordingStarting", "recordingStarted", "refreshThreads"]);
  expect(actions[1]).toEqual({ type: "recordingStarted", payload });
});

test("recording start failure recovers the meeting prompt", async () => {
  const actions: AppAction[] = [];
  let recovered = false;
  let refreshed = false;

  await runRecordingStart({
    dispatch: (action) => actions.push(action),
    start: async () => { throw new Error("Microphone unavailable"); },
    refreshThreads: async () => { refreshed = true; },
    recoverFailure: async () => { recovered = true; },
  });

  expect(actions.map((action) => action.type)).toEqual([
    "recordingStarting",
    "recordingStartFailed",
  ]);
  expect(recovered).toBe(true);
  expect(refreshed).toBe(false);
});

test("refresh failure does not turn a successful start into a start failure", async () => {
  const actions: AppAction[] = [];
  let recovered = false;

  await runRecordingStart({
    dispatch: (action) => actions.push(action),
    start: async () => payload,
    refreshThreads: async () => { throw new Error("Refresh failed"); },
    recoverFailure: async () => { recovered = true; },
  });

  expect(actions.map((action) => action.type)).toEqual([
    "recordingStarting",
    "recordingStarted",
    "failed",
  ]);
  expect(recovered).toBe(false);
});
