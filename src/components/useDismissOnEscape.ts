import { useEffect } from "react";

// Dismiss an overlay on Escape. macOS webviews never deliver keydown for Escape
// (tauri#5790), so listen on keyup as well.
export function useDismissOnEscape(onDismiss: () => void) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onDismiss();
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("keyup", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("keyup", onKey);
    };
  }, [onDismiss]);
}
