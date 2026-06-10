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
        ready: true,
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

  test("updates the thread list entry when a thread changes", () => {
    const renamed: ThreadDetail = {
      ...detail,
      summary: { ...summary, title: "Renamed" },
    };
    const state = { ...initialAppState, threads: [summary], selectedThread: detail };

    const updated = appReducer(state, { type: "threadUpdated", detail: renamed });

    expect(updated.selectedThread?.summary.title).toBe("Renamed");
    expect(updated.threads[0].title).toBe("Renamed");
  });

  test("clears the selection when the selected thread is deleted", () => {
    const state = {
      ...initialAppState,
      threads: [summary],
      selectedThreadId: "thread-1",
      selectedThread: detail,
    };

    const updated = appReducer(state, { type: "threadDeleted", threadId: "thread-1" });

    expect(updated.threads).toHaveLength(0);
    expect(updated.selectedThreadId).toBeNull();
    expect(updated.selectedThread).toBeNull();
  });

  test("stores finalization progress", () => {
    const updated = appReducer(initialAppState, {
      type: "finalizationReceived",
      payload: { threadId: "thread-1", state: "running", message: "Improving" },
    });

    expect(updated.finalization?.state).toBe("running");
  });
});
