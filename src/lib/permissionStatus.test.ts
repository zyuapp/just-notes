import { describe, expect, test } from "bun:test";
import type { MeetingAccessPayload } from "../bindings/MeetingAccessPayload";
import {
  meetingPromptsBlocked,
  permissionDisplay,
  recordingBlockers,
  settingsAttention,
} from "./permissionStatus";

function access(overrides: Partial<MeetingAccessPayload>): MeetingAccessPayload {
  return {
    calendarAuthorization: "authorized",
    notificationAuthorization: "authorized",
    calendars: [],
    ...overrides,
  };
}

describe("permissionDisplay", () => {
  test("granted access reads as healthy", () => {
    const display = permissionDisplay("authorized");
    expect(display.tone).toBe("ok");
    expect(display.needsAttention).toBe(false);
  });

  test("a refusal is an error, not a neutral note", () => {
    expect(permissionDisplay("denied").tone).toBe("danger");
    expect(permissionDisplay("denied").needsAttention).toBe(true);
    expect(permissionDisplay("restricted").needsAttention).toBe(true);
  });

  test("an unanswered prompt is not an error", () => {
    const display = permissionDisplay("notDetermined");
    expect(display.tone).toBe("idle");
    expect(display.needsAttention).toBe(false);
  });

  test("system audio's boolean preflight warns without claiming a refusal", () => {
    const display = permissionDisplay("notGranted");
    expect(display.tone).toBe("warning");
    expect(display.pill).toBe("Not granted");
    expect(display.needsAttention).toBe(true);
  });

  test("an unrecognised status stays neutral", () => {
    expect(permissionDisplay(undefined).tone).toBe("idle");
    expect(permissionDisplay("something-new").needsAttention).toBe(false);
  });
});

describe("recordingBlockers", () => {
  test("is empty when both permissions are granted", () => {
    expect(recordingBlockers({ microphone: "authorized", systemAudio: "authorized" })).toEqual([]);
  });

  test("names each permission standing in the way", () => {
    expect(recordingBlockers({ microphone: "denied", systemAudio: "authorized" })).toEqual([
      "Microphone",
    ]);
    expect(recordingBlockers({ microphone: "denied", systemAudio: "notGranted" })).toEqual([
      "Microphone",
      "System audio",
    ]);
  });
});

describe("meetingPromptsBlocked", () => {
  test("is false before calendars are connected — nothing is configured yet", () => {
    expect(meetingPromptsBlocked(access({ calendarAuthorization: "notDetermined" }))).toBe(false);
    expect(meetingPromptsBlocked(null)).toBe(false);
  });

  test("is true once calendars are connected but notifications cannot show", () => {
    expect(meetingPromptsBlocked(access({ notificationAuthorization: "denied" }))).toBe(true);
    expect(meetingPromptsBlocked(access({ notificationAuthorization: "notDetermined" }))).toBe(
      true,
    );
  });

  test("is false when both are granted", () => {
    expect(meetingPromptsBlocked(access({}))).toBe(false);
  });
});

describe("settingsAttention", () => {
  test("a nav badge is raised exactly when a section has a blocker", () => {
    expect(
      settingsAttention({ microphone: "authorized", systemAudio: "notGranted" }, access({})),
    ).toEqual({ permissions: true, meetings: false });
    expect(
      settingsAttention(
        { microphone: "authorized", systemAudio: "authorized" },
        access({ notificationAuthorization: "denied" }),
      ),
    ).toEqual({ permissions: false, meetings: true });
  });
});
