import { useEffect, useRef } from "react";
import { EscapeDismissStack } from "../lib/escapeDismissStack";

const dismissStack = new EscapeDismissStack();

function handleKeyDown(event: KeyboardEvent) {
  dismissStack.handleKeyDown(event);
}

function handleKeyUp(event: KeyboardEvent) {
  dismissStack.handleKeyUp(event);
}

// Dismiss an overlay on Escape. macOS webviews never deliver keydown for Escape
// (tauri#5790), so listen on keyup as well.
export function useDismissOnEscape(onDismiss: () => void) {
  const onDismissRef = useRef(onDismiss);
  onDismissRef.current = onDismiss;

  useEffect(() => {
    if (dismissStack.size === 0) {
      document.addEventListener("keydown", handleKeyDown, true);
      document.addEventListener("keyup", handleKeyUp, true);
    }
    const unregister = dismissStack.register(() => onDismissRef.current());
    return () => {
      unregister();
      if (dismissStack.size === 0) {
        document.removeEventListener("keydown", handleKeyDown, true);
        document.removeEventListener("keyup", handleKeyUp, true);
      }
    };
  }, []);
}
