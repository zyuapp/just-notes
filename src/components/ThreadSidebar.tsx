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
      <header className="sidebar-head">
        <div className="window-lights" aria-hidden="true">
          <span className="window-light red" />
          <span className="window-light yellow" />
          <span className="window-light green" />
        </div>
        <div className="sidebar-brand-row">
          <div className="brand">
            <BrandMark />
            <span>Just Notes</span>
          </div>
          <div className="sidebar-buttons">
            <button
              type="button"
              className="icon-button"
              onClick={onOpenSettings}
              aria-label="Settings"
            >
              <Settings size={22} aria-hidden="true" />
            </button>
            <button
              type="button"
              className="icon-button"
              onClick={onCreateThread}
              aria-label="New thread"
            >
              <Plus size={24} aria-hidden="true" />
            </button>
          </div>
        </div>
      </header>

      <div className="thread-search">
        <Search size={16} aria-hidden="true" />
        <input
          type="search"
          value={searchQuery}
          placeholder="Search recordings"
          onChange={(event) => onSearchChange(event.target.value)}
          aria-label="Search recordings"
        />
      </div>

      <ThreadList
        activeThreadId={activeThreadId}
        selectedThreadId={selectedThreadId}
        threads={threads}
        searching={searching}
        onSelectThread={onSelectThread}
      />

      <footer className="storage-path">
        <Folder size={24} aria-hidden="true" />
        <button type="button" onClick={onRevealStorage} title="Reveal in Finder">
          {appInfo?.threadsDir ?? "Locating storage…"}
        </button>
      </footer>
    </aside>
  );
}

function BrandMark() {
  return (
    <svg className="brand-mark" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path className="brand-mark-wave" d="M2.8 5.5c2.1 0 2.1-1.5 4.2-1.5s2.1 1.5 4.2 1.5S13.3 4 15.4 4s2.1 1.5 4.2 1.5" />
      <path className="brand-mark-wave" d="M2.8 11.7c2.1 0 2.1-1.5 4.2-1.5s2.1 1.5 4.2 1.5 2.1-1.5 4.2-1.5 2.1 1.5 4.2 1.5" />
      <path className="brand-mark-wave" d="M2.8 17.9c2.1 0 2.1-1.5 4.2-1.5s2.1 1.5 4.2 1.5 2.1-1.5 4.2-1.5 2.1 1.5 4.2 1.5" />
    </svg>
  );
}
