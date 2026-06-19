import { describe, expect, test } from "bun:test";
import { appReducer, initialAppState } from "./state";

const notice = { threadId: "thread-1", title: "Thread" };

describe("appReducer archive notice", () => {
  test("targeted archiveNoticeCleared only clears a notice for the same thread", () => {
    const state = { ...initialAppState, archivedNotice: notice };

    const mismatch = appReducer(state, { type: "archiveNoticeCleared", threadId: "thread-2" });
    expect(mismatch.archivedNotice).toEqual(notice);

    const match = appReducer(state, { type: "archiveNoticeCleared", threadId: "thread-1" });
    expect(match.archivedNotice).toBeNull();
  });

  test("untargeted archiveNoticeCleared always clears the notice", () => {
    const state = { ...initialAppState, archivedNotice: notice };

    expect(appReducer(state, { type: "archiveNoticeCleared" }).archivedNotice).toBeNull();
  });

  test("opening the archive view clears any pending undo notice", () => {
    const state = { ...initialAppState, archivedNotice: notice };

    const opened = appReducer(state, { type: "archiveOpenChanged", open: true });
    expect(opened.archiveOpen).toBe(true);
    expect(opened.archivedNotice).toBeNull();

    const closed = appReducer(opened, { type: "archiveOpenChanged", open: false });
    expect(closed.archiveOpen).toBe(false);
  });
});
