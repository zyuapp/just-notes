import { expect, mock, test } from "bun:test";
import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";
import { createMeetingAccessRefresh } from "./meetingAccessRefresh";

const oldAccess = {
  calendarAuthorization: "notDetermined",
  notificationAuthorization: "notDetermined",
  calendars: [],
} satisfies MeetingAccessPayload;

const currentAccess = {
  calendarAuthorization: "authorized",
  notificationAuthorization: "notDetermined",
  calendars: [{ id: "work", title: "Work" }],
} satisfies MeetingAccessPayload;

function deferredAccess() {
  let resolve: (access: MeetingAccessPayload) => void = () => undefined;
  const promise = new Promise<MeetingAccessPayload>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("the latest meeting access refresh wins", async () => {
  const onAccess = mock();
  const refresh = createMeetingAccessRefresh(onAccess);
  const first = deferredAccess();
  const second = deferredAccess();

  const firstLoad = refresh.refresh(() => first.promise);
  const secondLoad = refresh.refresh(() => second.promise);
  second.resolve(currentAccess);
  await secondLoad;
  first.resolve(oldAccess);
  await firstLoad;

  expect(onAccess.mock.calls).toEqual([[currentAccess]]);
});

test("a permission request invalidates and blocks automatic refreshes", async () => {
  const onAccess = mock();
  const load = mock(async () => currentAccess);
  const refresh = createMeetingAccessRefresh(onAccess);
  const pending = deferredAccess();
  const loading = refresh.refresh(() => pending.promise);

  refresh.beginPermissionRequest();
  await refresh.refresh(load);
  pending.resolve(oldAccess);
  await loading;

  expect(load).not.toHaveBeenCalled();
  expect(onAccess).not.toHaveBeenCalled();

  refresh.endPermissionRequest();
  await refresh.refresh(load);
  expect(onAccess.mock.calls).toEqual([[currentAccess]]);
});

test("invalidation prevents a late refresh from updating state", async () => {
  const onAccess = mock();
  const refresh = createMeetingAccessRefresh(onAccess);
  const pending = deferredAccess();
  const loading = refresh.refresh(() => pending.promise);

  refresh.invalidate();
  pending.resolve(oldAccess);
  await loading;

  expect(onAccess).not.toHaveBeenCalled();
});
