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

test("a refresh queued during a permission request runs when the request ends", async () => {
  let visibleAccess = oldAccess;
  const onAccess = mock((access: MeetingAccessPayload) => {
    visibleAccess = access;
  });
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
  expect(visibleAccess).toBe(oldAccess);

  await refresh.endPermissionRequest();
  expect(load).toHaveBeenCalledTimes(1);
  expect(onAccess.mock.calls).toEqual([[currentAccess]]);
  expect(visibleAccess).toBe(currentAccess);
});

test("multiple refreshes during one permission request coalesce", async () => {
  const onAccess = mock();
  const load = mock(async () => currentAccess);
  const refresh = createMeetingAccessRefresh(onAccess);

  refresh.beginPermissionRequest();
  await Promise.all([refresh.refresh(load), refresh.refresh(load), refresh.refresh(load)]);
  await refresh.endPermissionRequest();

  expect(load).toHaveBeenCalledTimes(1);
  expect(onAccess.mock.calls).toEqual([[currentAccess]]);
});

test("invalidation prevents a late queued refresh from updating state", async () => {
  const onAccess = mock();
  const refresh = createMeetingAccessRefresh(onAccess);
  const pending = deferredAccess();
  const load = mock(() => pending.promise);

  refresh.beginPermissionRequest();
  await refresh.refresh(load);
  const ending = refresh.endPermissionRequest();
  expect(load).toHaveBeenCalledTimes(1);

  refresh.invalidate();
  pending.resolve(currentAccess);
  await ending;

  expect(onAccess).not.toHaveBeenCalled();
});

test("invalidation clears a queued refresh before the request ends", async () => {
  const onAccess = mock();
  const load = mock(async () => currentAccess);
  const refresh = createMeetingAccessRefresh(onAccess);

  refresh.beginPermissionRequest();
  await refresh.refresh(load);
  refresh.invalidate();
  await refresh.endPermissionRequest();

  expect(load).not.toHaveBeenCalled();
  expect(onAccess).not.toHaveBeenCalled();
});

test("a failed queued refresh leaves the coordinator ready", async () => {
  const onAccess = mock();
  const refresh = createMeetingAccessRefresh(onAccess);

  refresh.beginPermissionRequest();
  await refresh.refresh(async () => {
    throw new Error("Status refresh failed");
  });
  await expect(refresh.endPermissionRequest()).rejects.toThrow("Status refresh failed");
  await refresh.refresh(async () => currentAccess);

  expect(onAccess.mock.calls).toEqual([[currentAccess]]);
});

test("ending without a queued refresh releases the request guard", async () => {
  const refresh = createMeetingAccessRefresh(mock());

  expect(refresh.beginPermissionRequest()).toBe(true);
  await refresh.endPermissionRequest();

  expect(refresh.beginPermissionRequest()).toBe(true);
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
