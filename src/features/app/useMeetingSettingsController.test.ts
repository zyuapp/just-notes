import { expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { api } from "../../api";
import type { MeetingAccessPayload } from "../../bindings/MeetingAccessPayload";
import { initialAppState, type AppAction } from "./state";
import { useMeetingSettingsController } from "./useMeetingSettingsController";

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

test("the controller drains focus refreshes, recovers, and cleans up", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previousWindow = globalThis.window;
  const previousDocument = globalThis.document;
  const previousNavigator = globalThis.navigator;
  const previousActEnvironment = globalThis.IS_REACT_ACT_ENVIRONMENT;
  const originalGetAccessStatus = api.meetings.getAccessStatus;
  const originalRequestCalendarAccess = api.meetings.requestCalendarAccess;
  const originalOnFocusChanged = api.window.onFocusChanged;
  Object.assign(globalThis, {
    window: browserWindow,
    document: browserWindow.document,
    navigator: browserWindow.navigator,
    IS_REACT_ACT_ENVIRONMENT: true,
  });

  const requests = [deferredAccess(), deferredAccess(), deferredAccess()];
  let requestIndex = 0;
  const requestCalendarAccess = mock(() => requests[requestIndex++].promise);
  let statusCall = 0;
  const getAccessStatus = mock(async () => {
    statusCall += 1;
    if (statusCall === 1) return oldAccess;
    if (statusCall === 2) return currentAccess;
    if (statusCall === 3) throw new Error("Status refresh failed");
    return currentAccess;
  });
  let focusChanged: ((focused: boolean) => void) | undefined;
  const unlisten = mock();
  api.meetings.getAccessStatus = getAccessStatus;
  api.meetings.requestCalendarAccess = requestCalendarAccess;
  api.window.onFocusChanged = mock(async (handler) => {
    focusChanged = handler;
    return unlisten;
  });

  const dispatched: AppAction[] = [];
  let controller: ReturnType<typeof useMeetingSettingsController> | undefined;
  let settingsOpen = true;
  function Harness() {
    controller = useMeetingSettingsController(
      { ...initialAppState, settingsOpen },
      (action) => dispatched.push(action),
      async () => undefined,
    );
    return null;
  }

  const root = createRoot(document.createElement("div"));
  try {
    await act(async () => {
      root.render(createElement(Harness));
      await new Promise<void>((resolve) => setImmediate(resolve));
    });
    expect(focusChanged).toBeDefined();
    expect(getAccessStatus).toHaveBeenCalledTimes(1);

    let firstRequest: Promise<void> | undefined;
    await act(async () => {
      firstRequest = controller?.requestCalendarAccess();
      await Promise.resolve();
      await controller?.requestCalendarAccess();
    });
    expect(requestCalendarAccess).toHaveBeenCalledTimes(1);
    await act(async () => {
      focusChanged?.(true);
      await Promise.resolve();
    });
    expect(getAccessStatus).toHaveBeenCalledTimes(1);

    requests[0].resolve(oldAccess);
    await act(async () => {
      await firstRequest;
      await new Promise<void>((resolve) => setImmediate(resolve));
    });
    const loadedAccess = dispatched
      .filter((action) => action.type === "meetingAccessLoaded")
      .map((action) => action.meetingAccess);
    expect(loadedAccess).toEqual([oldAccess, oldAccess, currentAccess]);

    let secondRequest: Promise<void> | undefined;
    await act(async () => {
      secondRequest = controller?.requestCalendarAccess();
      await Promise.resolve();
      focusChanged?.(true);
      requests[1].resolve(oldAccess);
      await secondRequest;
      await new Promise<void>((resolve) => setImmediate(resolve));
    });
    expect(getAccessStatus).toHaveBeenCalledTimes(3);
    expect(controller?.requestingAccess).toBe(false);

    let thirdRequest: Promise<void> | undefined;
    await act(async () => {
      thirdRequest = controller?.requestCalendarAccess();
      await Promise.resolve();
      focusChanged?.(true);
    });
    expect(requestCalendarAccess).toHaveBeenCalledTimes(3);
    settingsOpen = false;
    await act(async () => {
      root.render(createElement(Harness));
      await Promise.resolve();
    });
    expect(unlisten).toHaveBeenCalledTimes(1);
    await act(async () => {
      focusChanged?.(true);
      await Promise.resolve();
    });
    requests[2].resolve(currentAccess);
    await act(async () => {
      await thirdRequest;
      await new Promise<void>((resolve) => setImmediate(resolve));
    });
    expect(getAccessStatus).toHaveBeenCalledTimes(3);
  } finally {
    await act(async () => root.unmount());
    browserWindow.close();
    api.meetings.getAccessStatus = originalGetAccessStatus;
    api.meetings.requestCalendarAccess = originalRequestCalendarAccess;
    api.window.onFocusChanged = originalOnFocusChanged;
    Object.assign(globalThis, {
      window: previousWindow,
      document: previousDocument,
      navigator: previousNavigator,
      IS_REACT_ACT_ENVIRONMENT: previousActEnvironment,
    });
  }
});
