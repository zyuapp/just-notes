import {
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useRef,
  useState,
} from "react";
import {
  clampSidebarWidth,
  DEFAULT_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
  MIN_SIDEBAR_WIDTH,
} from "../../lib/sidebarWidth";

const STORAGE_KEY = "just-notes.sidebar-width";
const KEYBOARD_STEP = 16;

function readStoredWidth(): number {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw === null) return DEFAULT_SIDEBAR_WIDTH;
    return clampSidebarWidth(Number.parseInt(raw, 10));
  } catch {
    return DEFAULT_SIDEBAR_WIDTH;
  }
}

function writeStoredWidth(width: number): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, String(width));
  } catch {
    // localStorage may be unavailable
  }
}

export function useSidebarWidth() {
  const [width, setWidth] = useState(readStoredWidth);
  const widthRef = useRef(width);
  widthRef.current = width;
  const draggingRef = useRef(false);

  const commitWidth = useCallback((next: number) => {
    const clamped = clampSidebarWidth(next);
    setWidth(clamped);
    writeStoredWidth(clamped);
  }, []);

  const onResizeStart = useCallback((event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0 || draggingRef.current) return;
    event.preventDefault();
    draggingRef.current = true;
    const { pointerId } = event;
    const startX = event.clientX;
    const startWidth = widthRef.current;
    let nextWidth = startWidth;

    const prevCursor = document.body.style.cursor;
    const prevSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMove = (move: PointerEvent) => {
      if (move.pointerId !== pointerId) return;
      nextWidth = clampSidebarWidth(startWidth + (move.clientX - startX));
      setWidth(nextWidth);
    };
    const onStop = (end: PointerEvent) => {
      if (end.pointerId !== pointerId) return;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onStop);
      window.removeEventListener("pointercancel", onStop);
      document.body.style.cursor = prevCursor;
      document.body.style.userSelect = prevSelect;
      draggingRef.current = false;
      writeStoredWidth(nextWidth);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onStop);
    window.addEventListener("pointercancel", onStop);
  }, []);

  const onResizeKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLElement>) => {
      const current = widthRef.current;
      let next: number | null = null;
      if (event.key === "ArrowLeft") next = current - KEYBOARD_STEP;
      else if (event.key === "ArrowRight") next = current + KEYBOARD_STEP;
      else if (event.key === "Home") next = MIN_SIDEBAR_WIDTH;
      else if (event.key === "End") next = MAX_SIDEBAR_WIDTH;
      if (next === null) return;
      event.preventDefault();
      commitWidth(next);
    },
    [commitWidth],
  );

  const resetWidth = useCallback(() => commitWidth(DEFAULT_SIDEBAR_WIDTH), [commitWidth]);

  return { width, onResizeStart, onResizeKeyDown, resetWidth };
}
