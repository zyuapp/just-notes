import { Archive, Plus, Settings } from "lucide-react";
import { type RefObject, useState } from "react";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { Button } from "./Button";
import { SearchField } from "./SearchField";
import { ThreadContextMenu } from "./ThreadContextMenu";
import { ThreadList } from "./ThreadList";

type ThreadSidebarProps = {
  activeThreadId: string | null;
  selectedThreadId: string | null;
  threads: ThreadSummary[];
  searchQuery: string;
  searching: boolean;
  // Owned by App so the transcript toolbar's archive triggers the same flight.
  iconRef: RefObject<HTMLButtonElement | null>;
  scopeRef: RefObject<HTMLElement | null>;
  onSearchChange: (query: string) => void;
  onCreateThread: () => void;
  onSelectThread: (threadId: string) => void;
  onExportThread: (threadId: string) => void;
  onRevealThread: (path: string) => void;
  onArchiveThread: (threadId: string) => void;
  onOpenArchive: () => void;
  onOpenSettings: () => void;
};

type ThreadMenuState = { thread: ThreadSummary; x: number; y: number };

export function ThreadSidebar({
  activeThreadId,
  selectedThreadId,
  threads,
  searchQuery,
  searching,
  iconRef,
  scopeRef,
  onSearchChange,
  onCreateThread,
  onSelectThread,
  onExportThread,
  onRevealThread,
  onArchiveThread,
  onOpenArchive,
  onOpenSettings,
}: ThreadSidebarProps) {
  const [menu, setMenu] = useState<ThreadMenuState | null>(null);

  return (
    <aside className="thread-sidebar" aria-label="Threads" ref={scopeRef}>
      <div className="sidebar-drag" data-tauri-drag-region="" />
      <header className="sidebar-head" data-tauri-drag-region="">
        <div className="brand">
          <BrandMark />
          <span>Just Notes</span>
        </div>
        <Button
          variant="icon"
          onClick={onCreateThread}
          aria-label="New thread"
          title="New thread"
        >
          <Plus size={15} aria-hidden="true" />
        </Button>
      </header>

      <SearchField
        className="thread-search"
        value={searchQuery}
        placeholder="Search recordings"
        onChange={onSearchChange}
      />

      <ThreadList
        activeThreadId={activeThreadId}
        selectedThreadId={selectedThreadId}
        threads={threads}
        searching={searching}
        onSelectThread={onSelectThread}
        onOpenMenu={(thread, x, y) => setMenu({ thread, x, y })}
      />

      <footer className="sidebar-foot">
        <Button
          ref={iconRef}
          variant="icon"
          onClick={onOpenArchive}
          aria-label="Archived recordings"
          title="Archived recordings"
        >
          <Archive size={15} aria-hidden="true" />
        </Button>
        <Button
          variant="icon"
          onClick={onOpenSettings}
          aria-label="Settings"
          title="Settings"
        >
          <Settings size={15} aria-hidden="true" />
        </Button>
      </footer>

      {menu && (
        <ThreadContextMenu
          thread={menu.thread}
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          onExport={onExportThread}
          onReveal={onRevealThread}
          onArchive={onArchiveThread}
        />
      )}
    </aside>
  );
}

function BrandMark() {
  return (
    <svg
      className="brand-mark"
      viewBox="0 0 16 16"
      width="15"
      height="15"
      aria-hidden="true"
      focusable="false"
    >
      <g fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
        <path d="M2 8h.01M5 5.5v5M8 3.5v9M11 5.5v5M14 8h.01" />
      </g>
    </svg>
  );
}
