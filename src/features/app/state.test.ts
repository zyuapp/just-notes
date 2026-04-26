import { describe, expect, test } from "bun:test";
import type { LiveTranscriptSegmentPayload } from "../../bindings/LiveTranscriptSegmentPayload";
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
};

const detail: ThreadDetail = {
  summary,
  segments: [],
  transcriptMarkdownPath: "/tmp/transcript.md",
  transcriptJsonlPath: "/tmp/transcript.jsonl",
};

function segment(source: string, startMs: number): TranscriptSegment {
  return {
    source,
    speaker: source === "mic" ? "You" : "Others",
    startMs,
    endMs: startMs + 1000,
    text: source,
  };
}

describe("appReducer", () => {
  test("selects threads from details", () => {
    const state = appReducer(initialAppState, { type: "threadSelected", detail });

    expect(state.selectedThreadId).toBe("thread-1");
    expect(state.selectedThread).toBe(detail);
  });

  test("marks recording as active after recording starts", () => {
    const payload: RecordingPayload = {
      thread: detail,
      transcription: {
        engineExists: true,
        modelExists: true,
        enginePath: "/tmp/engine",
        modelPath: "/tmp/model",
        modelName: "small.en",
        availableModels: [],
        message: "Ready",
      },
    };

    const state = appReducer(initialAppState, { type: "recordingStarted", payload });

    expect(state.recorderState).toBe("recording");
    expect(state.selectedThreadId).toBe("thread-1");
    expect(state.transcriptionStatus?.message).toBe("Ready");
  });

  test("applies live transcript segments to selected thread and thread list", () => {
    const payload: LiveTranscriptSegmentPayload = {
      threadId: "thread-1",
      committedUntilMs: 4000,
      segment: segment("mic", 3000),
    };
    const state = {
      ...initialAppState,
      selectedThread: {
        ...detail,
        segments: [segment("system", 5000)],
      },
      threads: [summary],
    };

    const updated = appReducer(state, { type: "liveSegmentReceived", payload, updatedAtMs: 200 });

    expect(updated.selectedThread?.segments.map((item) => `${item.startMs}:${item.source}`)).toEqual([
      "3000:mic",
      "5000:system",
    ]);
    expect(updated.threads[0].segmentCount).toBe(1);
    expect(updated.threads[0].updatedAtMs).toBe(200);
  });
});
