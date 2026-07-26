import { Archive, FileDown, FolderOpen } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ThreadSummary } from "../bindings/ThreadSummary";

type ThreadContextMenuProps = {
  thread: ThreadSummary;
  x: number;
  y: number;
  onClose: () => void;
  onExport: (threadId: string) => void;
  onReveal: (path: string) => void;
  onArchive: (threadId: string) => void;
};

const EDGE_GAP = 8;

export function ThreadContextMenu({
  thread,
  x,
  y,
  onClose,
  onExport,
  onReveal,
  onArchive,
}: ThreadContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const { width, height } = menu.getBoundingClientRect();
    setPos({
      x: Math.max(EDGE_GAP, Math.min(x, window.innerWidth - width - EDGE_GAP)),
      y: Math.max(EDGE_GAP, Math.min(y, window.innerHeight - height - EDGE_GAP)),
    });
  }, [x, y]);

  useEffect(() => {
    const dismiss = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", dismiss);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", dismiss);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="thread-menu"
      role="menu"
      style={{ left: pos.x, top: pos.y }}
    >
      <button
        type="button"
        role="menuitem"
        className="thread-menu-item"
        onClick={() => {
          onExport(thread.id);
          onClose();
        }}
      >
        <FileDown size={14} aria-hidden="true" />
        <span>Export Markdown</span>
      </button>
      <button
        type="button"
        role="menuitem"
        className="thread-menu-item"
        onClick={() => {
          onReveal(thread.path);
          onClose();
        }}
      >
        <FolderOpen size={14} aria-hidden="true" />
        <span>Show in Finder</span>
      </button>
      <div className="thread-menu-divider" />
      <button
        type="button"
        role="menuitem"
        className="thread-menu-item"
        onClick={() => {
          onArchive(thread.id);
          onClose();
        }}
      >
        <Archive size={14} aria-hidden="true" />
        <span>Archive</span>
      </button>
    </div>
  );
}
