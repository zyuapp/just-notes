import { Folder, Plus, Search, Settings } from "lucide-react";
import type { AppInfo } from "../bindings/AppInfo";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { ThreadList } from "./ThreadList";

type ThreadSidebarProps = {
  activeThreadId: string | null;
  appInfo: AppInfo | null;
  selectedThreadId: string | null;
  threads: ThreadSummary[];
  searchQuery: string;
  searching: boolean;
  onSearchChange: (query: string) => void;
  onCreateThread: () => void;
  onSelectThread: (threadId: string) => void;
  onOpenSettings: () => void;
  onRevealStorage: () => void;
};

export function ThreadSidebar({
  activeThreadId,
  appInfo,
  selectedThreadId,
  threads,
  searchQuery,
  searching,
  onSearchChange,
  onCreateThread,
  onSelectThread,
  onOpenSettings,
  onRevealStorage,
}: ThreadSidebarProps) {
  return (
    <aside className="thread-sidebar" aria-label="Threads">
      <div className="sidebar-drag" data-tauri-drag-region="" />
      <header className="sidebar-head" data-tauri-drag-region="">
        <div className="brand">
          <BrandMark />
          <span>Just Notes</span>
        </div>
        <button
          type="button"
          className="icon-button"
          onClick={onCreateThread}
          aria-label="New thread"
        >
          <Plus size={15} aria-hidden="true" />
        </button>
      </header>

      <label className="thread-search">
        <Search size={13} aria-hidden="true" />
        <input
          type="search"
          value={searchQuery}
          placeholder="Search recordings"
          onChange={(event) => onSearchChange(event.target.value)}
          aria-label="Search recordings"
        />
      </label>

      <ThreadList
        activeThreadId={activeThreadId}
        selectedThreadId={selectedThreadId}
        threads={threads}
        searching={searching}
        onSelectThread={onSelectThread}
      />

      <footer className="sidebar-foot">
        <button
          type="button"
          className="icon-button"
          onClick={onOpenSettings}
          aria-label="Settings"
        >
          <Settings size={15} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="storage-path"
          onClick={onRevealStorage}
          title="Reveal in Finder"
        >
          <Folder size={13} aria-hidden="true" />
          <span>{appInfo?.threadsDir ?? "Locating storage…"}</span>
        </button>
      </footer>
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
