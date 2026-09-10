import { expect, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AppearanceProvider, APPEARANCE_STORAGE_KEY } from "../features/appearance/AppearanceProvider";
import { AppearanceSettingsSection } from "./AppearanceSettingsSection";

test("appearance follows the system outside Settings, persists overrides, and cleans up listeners", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previous = { window: globalThis.window, document: globalThis.document,
    navigator: globalThis.navigator, IS_REACT_ACT_ENVIRONMENT: globalThis.IS_REACT_ACT_ENVIRONMENT };
  Object.assign(globalThis, { window: browserWindow, document: browserWindow.document,
    navigator: browserWindow.navigator, IS_REACT_ACT_ENVIRONMENT: true });
  let systemDark = true;
  const listeners = new Set<() => void>();
  const mediaQuery = {
    get matches() { return systemDark; },
    addEventListener: (_event: string, listener: () => void) => listeners.add(listener),
    removeEventListener: (_event: string, listener: () => void) => listeners.delete(listener),
  };
  Object.defineProperty(browserWindow, "matchMedia", { value: () => mediaQuery });
  const container = document.createElement("div");
  const root = createRoot(container);
  const render = async (settingsOpen = true) => act(async () => root.render(
    createElement(StrictMode, null, createElement(AppearanceProvider, null,
      settingsOpen ? createElement(AppearanceSettingsSection) : null))));
  const choose = async (label: string) => act(async () => {
    [...container.querySelectorAll("button")].find(button => button.textContent === label)!.click();
  });
  const changeSystem = async (dark: boolean) => act(async () => {
    systemDark = dark;
    listeners.forEach(listener => listener());
  });
  const appearance = () => document.documentElement.dataset.appearance;
  const selected = () => container.querySelector('[aria-pressed="true"]')?.textContent;

  try {
    window.localStorage.setItem("just-notes.appearance", "light");
    await render();
    expect(appearance()).toBe("dark");
    expect(selected()).toBe("System");
    expect(listeners.size).toBe(1);
    await changeSystem(false);
    expect(appearance()).toBe("light");

    await choose("Light");
    expect(window.localStorage.getItem(APPEARANCE_STORAGE_KEY)).toBe("light");
    await changeSystem(true);
    expect(appearance()).toBe("light");
    expect(selected()).toBe("Light");

    await choose("Dark");
    expect(window.localStorage.getItem(APPEARANCE_STORAGE_KEY)).toBe("dark");
    await changeSystem(false);
    expect(appearance()).toBe("dark");
    await act(async () => root.render(null));
    expect(listeners.size).toBe(0);
    await render();
    expect(appearance()).toBe("dark");
    expect(selected()).toBe("Dark");

    await choose("System");
    expect(appearance()).toBe("light");
    expect(window.localStorage.getItem(APPEARANCE_STORAGE_KEY)).toBe("system");
    await render(false);
    await changeSystem(true);
    expect(appearance()).toBe("dark");
    await render();
    expect(selected()).toBe("System");
    await act(async () => root.render(null));
    await render();
    expect(selected()).toBe("System");
    expect(appearance()).toBe("dark");

    await act(async () => root.render(null));
    window.localStorage.setItem(APPEARANCE_STORAGE_KEY, "invalid preference");
    await render();
    expect(selected()).toBe("System");
    await act(async () => root.render(null));
    Object.defineProperty(browserWindow, "localStorage", {
      configurable: true, get() { throw new Error("Storage unavailable"); },
    });
    await render();
    expect(selected()).toBe("System");
    await choose("Light");
    expect(appearance()).toBe("light");
    await choose("System");
    expect(appearance()).toBe("dark");
  } finally {
    await act(async () => root.unmount());
    expect(listeners.size).toBe(0);
    browserWindow.close();
    Object.assign(globalThis, previous);
  }
});
