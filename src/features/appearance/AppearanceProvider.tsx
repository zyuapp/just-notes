import { createContext, type ReactNode, useContext, useLayoutEffect, useState } from "react";

export type AppearancePreference = "system" | "light" | "dark";

// The old toggle saved its light default automatically. A separate key lets
// everyone start with System while preserving explicit choices made here.
export const APPEARANCE_STORAGE_KEY = "just-notes.appearance-preference";
const systemQuery = "(prefers-color-scheme: dark)";

type AppearanceState = {
  preference: AppearancePreference;
  setPreference: (preference: AppearancePreference) => void;
};

const AppearanceContext = createContext<AppearanceState | null>(null);

function storedPreference(): AppearancePreference {
  try {
    const stored = window.localStorage.getItem(APPEARANCE_STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {
    // System appearance also works when storage is unavailable.
  }
  return "system";
}

export function AppearanceProvider({ children }: { children?: ReactNode }) {
  const [preference, updatePreference] = useState(storedPreference);
  const [systemDark, setSystemDark] = useState(() => window.matchMedia(systemQuery).matches);

  useLayoutEffect(() => {
    const query = window.matchMedia(systemQuery);
    const update = () => setSystemDark(query.matches);
    query.addEventListener("change", update);
    update();
    return () => query.removeEventListener("change", update);
  }, []);

  useLayoutEffect(() => {
    document.documentElement.dataset.appearance = preference === "system"
      ? systemDark ? "dark" : "light"
      : preference;
  }, [preference, systemDark]);

  const setPreference = (next: AppearancePreference) => {
    updatePreference(next);
    try {
      window.localStorage.setItem(APPEARANCE_STORAGE_KEY, next);
    } catch {
      // Apply the choice for this launch even when it cannot be persisted.
    }
  };

  return (
    <AppearanceContext.Provider value={{ preference, setPreference }}>
      {children}
    </AppearanceContext.Provider>
  );
}

export function useAppearance() {
  const context = useContext(AppearanceContext);
  if (!context) throw new Error("AppearanceProvider is missing");
  return context;
}
