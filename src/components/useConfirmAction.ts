import { useEffect, useState } from "react";

const CONFIRM_TIMEOUT_MS = 4000;

// Two-step confirm for a destructive action: the first trigger arms, a second
// trigger within the timeout commits. macOS WebKit does not focus buttons on
// click, so the armed state cannot rely on blur alone — it auto-disarms on a timer.
export function useConfirmAction(commit: () => void) {
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), CONFIRM_TIMEOUT_MS);
    return () => clearTimeout(timer);
  }, [armed]);

  const trigger = () => {
    if (armed) {
      setArmed(false);
      commit();
    } else {
      setArmed(true);
    }
  };

  return { armed, trigger, reset: () => setArmed(false) };
}
