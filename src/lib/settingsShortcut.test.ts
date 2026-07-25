import { describe, expect, test } from "bun:test";
import { isSettingsShortcut } from "./settingsShortcut";

function press(overrides: Partial<Parameters<typeof isSettingsShortcut>[0]>) {
  return isSettingsShortcut({
    metaKey: true,
    key: ",",
    altKey: false,
    ctrlKey: false,
    ...overrides,
  });
}

describe("isSettingsShortcut", () => {
  test("matches the macOS settings shortcut", () => {
    expect(press({})).toBe(true);
  });

  test("ignores a bare comma", () => {
    expect(press({ metaKey: false })).toBe(false);
  });

  test("ignores other keys and wider chords", () => {
    expect(press({ key: "." })).toBe(false);
    expect(press({ altKey: true })).toBe(false);
    expect(press({ ctrlKey: true })).toBe(false);
  });
});
