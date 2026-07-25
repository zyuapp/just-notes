import { useEffect, useRef } from "react";

// Runs `refresh` when `active` turns true and again whenever the window regains
// focus while it stays true. Permissions and disk usage both change outside the
// app, so returning to Just Notes is the only signal that they may have moved.
export function useWindowFocusRefresh(active: boolean, refresh: () => void) {
  const refreshRef = useRef(refresh);
  refreshRef.current = refresh;

  useEffect(() => {
    if (!active) return;
    refreshRef.current();
    // `focus` alone: on macOS it also fires when the window is un-occluded, so
    // pairing it with `visibilitychange` would just double every refresh.
    const onFocus = () => refreshRef.current();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [active]);
}
