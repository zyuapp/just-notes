import { expect, test } from "bun:test";
import type { MeetingPromptPayload } from "../../bindings/MeetingPromptPayload";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { ThreadDetail } from "../../bindings/ThreadDetail";
import { appReducer, initialAppState } from "./state";

const prompt: MeetingPromptPayload = {
  requestId: "meeting-prompt-1",
  title: "Design review",
  startAtMs: 200,
  endAtMs: 300,
};

const thread: ThreadDetail = {
  summary: {
    id: "thread-1", title: "Design review", createdAtMs: 100, updatedAtMs: 100,
    segmentCount: 0, status: "recording", durationMs: 0, snippet: "", hasAudio: false,
    path: "/threads/thread-1", calendar: null,
  },
  segments: [],
  transcriptMarkdownPath: "/threads/thread-1/transcript.md",
};

test("starting a recording leaves prompt ownership to the meetings domain", () => {
  const payload = {
    thread,
    transcription: {
      ready: true, engineExists: true, modelExists: true, enginePath: "", modelPath: "",
      modelName: "Parakeet", availableModels: [], message: "Ready",
    },
  } satisfies RecordingPayload;

  const state = appReducer(
    { ...initialAppState, meetingPrompt: prompt },
    { type: "recordingStarted", payload },
  );

  expect(state.meetingPrompt).toEqual(prompt);
});

test("a stale start failure cannot overwrite an active recording", () => {
  const payload = {
    thread,
    transcription: {
      ready: true, engineExists: true, modelExists: true, enginePath: "", modelPath: "",
      modelName: "Parakeet", availableModels: [], message: "Ready",
    },
  } satisfies RecordingPayload;
  const recording = appReducer(
    { ...initialAppState, recorderState: "starting" },
    { type: "recordingStarted", payload },
  );

  const afterLateFailure = appReducer(recording, {
    type: "recordingStartFailed",
    message: "Meeting prompt is no longer available",
  });

  expect(afterLateFailure.recorderState).toBe("recording");
  expect(afterLateFailure.recordingThreadId).toBe("thread-1");
});
