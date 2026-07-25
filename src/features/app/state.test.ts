import { describe, expect, test } from "bun:test";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { ThreadDetail } from "../../bindings/ThreadDetail";
import type { ThreadSummary } from "../../bindings/ThreadSummary";
import { appReducer, getActiveThreadId, initialAppState } from "./state";

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
  transcriptMarkdownPath: "/tmp/transcript.md",
};

const otherSummary: ThreadSummary = {
  ...summary,
  id: "thread-2",
  title: "Other thread",
  path: "/threads/thread-2",
};

const otherDetail: ThreadDetail = {
  ...detail,
  summary: otherSummary,
};

const transcriptionStatus = {
  ready: true,
  engineExists: true,
  modelExists: true,
  enginePath: "/tmp/engine",
  modelPath: "/tmp/model",
  modelName: "Parakeet TDT 0.6B v2",
  availableModels: [],
  message: "Ready",
};

describe("appReducer", () => {
  test("selects threads from details", () => {
    const state = appReducer(initialAppState, { type: "threadSelected", detail });

    expect(state.selectedThreadId).toBe("thread-1");
    expect(state.selectedThread).toBe(detail);
  });

  test("marks recording as active after recording starts", () => {
    const payload: RecordingPayload = {
      thread: detail,
      transcription: transcriptionStatus,
    };

    const state = appReducer(initialAppState, { type: "recordingStarted", payload });

    expect(state.recorderState).toBe("recording");
    expect(state.selectedThreadId).toBe("thread-1");
    expect(state.recordingThreadId).toBe("thread-1");
    expect(getActiveThreadId(state)).toBe("thread-1");
    expect(state.transcriptionStatus?.message).toBe("Ready");
  });

  test("keeps the active recording thread stable while selecting another thread", () => {
    const payload: RecordingPayload = {
      thread: detail,
      transcription: transcriptionStatus,
    };
    const recording = appReducer(
      { ...initialAppState, threads: [summary, otherSummary] },
      { type: "recordingStarted", payload },
    );

    const navigated = appReducer(recording, { type: "threadSelected", detail: otherDetail });

    expect(navigated.selectedThreadId).toBe("thread-2");
    expect(navigated.recordingThreadId).toBe("thread-1");
    expect(getActiveThreadId(navigated)).toBe("thread-1");
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

  test("removes the thread and clears selection when archived", () => {
    const state = {
      ...initialAppState,
      threads: [summary],
      selectedThreadId: "thread-1",
      selectedThread: detail,
    };

    const updated = appReducer(state, { type: "threadArchived", summary });

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

  test("stores meeting calendar access", () => {
    const meetingAccess = { calendarAuthorization: "authorized",
      notificationAuthorization: "authorized", calendars: [{ id: "calendar-1", title: "Work", account: "iCloud", color: "#5b8ff9" }] };

    const updated = appReducer(initialAppState, {
      type: "meetingAccessLoaded",
      meetingAccess,
    });
    expect(updated.meetingAccess).toBe(meetingAccess);
  });

  test("updates transcription status without changing selected thread", () => {
    const state = {
      ...initialAppState,
      threads: [summary, otherSummary],
      selectedThreadId: "thread-2",
      selectedThread: otherDetail,
      transcriptionStatus,
    };
    const updatedStatus = {
      ...transcriptionStatus,
      modelName: "Parakeet TDT 0.6B v2 (updated)",
    };

    const updated = appReducer(state, {
      type: "transcriptionStatusLoaded",
      transcriptionStatus: updatedStatus,
    });

    expect(updated.selectedThreadId).toBe("thread-2");
    expect(updated.selectedThread).toBe(otherDetail);
    expect(updated.transcriptionStatus?.modelName).toBe("Parakeet TDT 0.6B v2 (updated)");
  });
});
