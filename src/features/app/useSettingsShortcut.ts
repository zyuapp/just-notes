import { useEffect, useRef } from "react";
import { isSettingsShortcut } from "../../lib/settingsShortcut";

// ⌘, is the standard macOS shortcut for an app's settings, and it is the first
// thing a Mac user reaches for.
export function useSettingsShortcut(openSettings: () => void) {
  const openRef = useRef(openSettings);
  openRef.current = openSettings;

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!isSettingsShortcut(event)) return;
      event.preventDefault();
      openRef.current();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);
}
