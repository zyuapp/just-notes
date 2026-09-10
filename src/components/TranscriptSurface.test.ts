import { expect, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import { TranscriptSurface } from "./TranscriptSurface";

const segment = (startMs: number, source: string, text: string): TranscriptSegment => ({
  source, speaker: source === "mic" ? "You" : "Others", startMs, endMs: startMs + 500, text,
});

test("flowing speaker turns preserve all fragments, filtered boundaries, and live deduplication", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previous = { window: globalThis.window, document: globalThis.document,
    navigator: globalThis.navigator, IS_REACT_ACT_ENVIRONMENT: globalThis.IS_REACT_ACT_ENVIRONMENT };
  Object.assign(globalThis, { window: browserWindow, document: browserWindow.document,
    navigator: browserWindow.navigator, IS_REACT_ACT_ENVIRONMENT: true });
  const container = document.createElement("div");
  const root = createRoot(container);
  const segments = [
    segment(0, "mic", "Can you hear me?"),
    segment(1_000, "mic", "Checking the microphone."),
    segment(3_000, "system", "Yes, I can hear you."),
    segment(4_000, "mic", "Keep this first part."),
    segment(5_000, "mic", "A different topic."),
    segment(6_000, "mic", "Keep the final part."),
  ];
  const detail: ThreadDetail = {
    summary: { id: "note", title: "Conversation", createdAtMs: 0, updatedAtMs: 0,
      durationMs: 6_500, segmentCount: segments.length, status: "idle", snippet: "",
      hasAudio: false, path: "/sample", calendar: null },
    segments, transcriptMarkdownPath: "/sample/transcript.md",
  };
  const render = async (query = "", liveSegments: TranscriptSegment[] = []) => act(async () => {
    root.render(createElement(TranscriptSurface, { selectedThread: detail, query, liveSegments,
      recorderState: liveSegments.length ? "recording" : "idle", transcriptionStatus: null }));
  });
  try {
    await render();
    expect([...container.querySelectorAll(".segment p")].map(p => p.textContent)).toEqual([
      "Can you hear me? Checking the microphone.",
      "Yes, I can hear you.",
      "Keep this first part. A different topic. Keep the final part.",
    ]);
    expect([...container.querySelectorAll(".segment-head strong")].map(p => p.textContent))
      .toEqual(["You", "Others", "You"]);
    expect(container.querySelectorAll(".segment-text")).toHaveLength(segments.length);

    await render("  KEEP  ");
    expect(container.querySelectorAll(".segment")).toHaveLength(2);
    expect([...container.querySelectorAll(".segment-head time")].map(p => p.textContent))
      .toEqual(["00:04", "00:06"]);
    expect([...container.querySelectorAll("mark")].map(p => p.textContent)).toEqual(["Keep", "Keep"]);
    expect([...container.querySelectorAll(".segment p")].map(p => p.textContent))
      .toEqual(["Keep this first part.", "Keep the final part."]);

    await render("unmatched query");
    expect(container.querySelector(".transcript-no-match")?.textContent).toContain("unmatched query");

    await render("", [segments[5], segment(7_000, "mic", "An incoming fragment.")]);
    expect(container.querySelectorAll(".segment-text")).toHaveLength(segments.length + 1);
    expect(container.querySelectorAll(".segment")).toHaveLength(3);
    expect(container.querySelector(".segment-text.active")?.textContent).toBe(" An incoming fragment.");
    expect(container.querySelector(".segment:last-child p")?.textContent)
      .toBe("Keep this first part. A different topic. Keep the final part. An incoming fragment.");
  } finally {
    await act(async () => root.unmount());
    browserWindow.close();
    Object.assign(globalThis, previous);
  }
});
