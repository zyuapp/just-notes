import { describe, expect, test } from "bun:test";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { ThreadDetail } from "../../bindings/ThreadDetail";
import type { ThreadSummary } from "../../bindings/ThreadSummary";
import type { TranscriptSegment } from "../../bindings/TranscriptSegment";
import { appReducer, initialAppState } from "./state";

const summary: ThreadSummary = {
  id: "thread-1",
  title: "Thread",
  createdAtMs: 100,
  updatedAtMs: 100,
  segmentCount: 0,
  status: "idle",
  durationMs: 0,
  snippet: "",
  hasAudio: false,
  path: "/threads/thread-1",
};

const detail: ThreadDetail = {
  summary,
  segments: [],
  speakerLabels: {},
  transcriptMarkdownPath: "/tmp/transcript.md",
};

const transcription = {
  ready: true,
  engineExists: true,
  modelExists: true,
  enginePath: "/tmp/engine",
  modelPath: "/tmp/model",
  modelName: "Parakeet",
  availableModels: [],
  message: "Ready",
};

const liveSegment: TranscriptSegment = {
  source: "mic",
  speaker: "You",
  startMs: 1_000,
  endMs: 2_000,
  text: "hello",
};

function recordingState() {
  const payload: RecordingPayload = { thread: detail, transcription };
  return appReducer(initialAppState, { type: "recordingStarted", payload });
}

describe("appReducer liveSegmentReceived", () => {
  test("appends a live segment while recording the matching thread", () => {
    const updated = appReducer(recordingState(), {
      type: "liveSegmentReceived",
      payload: { threadId: "thread-1", segment: liveSegment },
    });

    expect(updated.liveSegments).toEqual([liveSegment]);
  });

  test("ignores a live segment for a thread other than the one recording", () => {
    const base = recordingState();
    const updated = appReducer(base, {
      type: "liveSegmentReceived",
      payload: { threadId: "thread-2", segment: liveSegment },
    });

    expect(updated).toBe(base);
  });

  test("ignores a live segment when not recording", () => {
    const updated = appReducer(initialAppState, {
      type: "liveSegmentReceived",
      payload: { threadId: "thread-1", segment: liveSegment },
    });

    expect(updated).toBe(initialAppState);
  });
});
