import { expect, test } from "bun:test";
import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";
import { runMeetingAccessRequest } from "./meetingAccessRequest";
import type { AppAction } from "./state";

const authorized = {
  calendarAuthorization: "authorized",
  notificationAuthorization: "notDetermined",
  calendars: [{ id: "work", title: "Work" }],
} satisfies MeetingAccessPayload;

const denied = {
  calendarAuthorization: "denied",
  notificationAuthorization: "notDetermined",
  calendars: [],
} satisfies MeetingAccessPayload;

test("meeting access success dispatches the returned state immediately", async () => {
  const actions: AppAction[] = [];
  let refreshCalled = false;

  await runMeetingAccessRequest({
    dispatch: (action) => actions.push(action),
    request: async () => authorized,
    refresh: async () => {
      refreshCalled = true;
      return denied;
    },
  });

  expect(actions).toEqual([
    { type: "errorCleared" },
    { type: "meetingAccessLoaded", meetingAccess: authorized },
  ]);
  expect(refreshCalled).toBe(false);
});

test("request failure refreshes state before surfacing the original error", async () => {
  const actions: AppAction[] = [];

  await runMeetingAccessRequest({
    dispatch: (action) => actions.push(action),
    request: async () => {
      throw new Error("Calendar request failed");
    },
    refresh: async () => authorized,
  });

  expect(actions).toEqual([
    { type: "errorCleared" },
    { type: "meetingAccessLoaded", meetingAccess: authorized },
    { type: "failed", message: "Calendar request failed" },
  ]);
});

test("fallback failure does not replace the permission-request error", async () => {
  const actions: AppAction[] = [];

  await runMeetingAccessRequest({
    dispatch: (action) => actions.push(action),
    request: async () => {
      throw new Error("Notification request failed");
    },
    refresh: async () => {
      throw new Error("Status refresh failed");
    },
  });

  expect(actions).toEqual([
    { type: "errorCleared" },
    { type: "failed", message: "Notification request failed" },
  ]);
});
