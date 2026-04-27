import { Folder, Plus } from "lucide-react";
import type { AppInfo } from "../bindings/AppInfo";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { ThreadList } from "./ThreadList";

type ThreadSidebarProps = {
  activeThreadId: string | null;
  appInfo: AppInfo | null;
  selectedThreadId: string | null;
  threads: ThreadSummary[];
  onCreateThread: () => void;
  onSelectThread: (threadId: string) => void;
};

export function ThreadSidebar({
  activeThreadId,
  appInfo,
  selectedThreadId,
  threads,
  onCreateThread,
  onSelectThread,
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
          <button type="button" className="icon-button" onClick={onCreateThread} aria-label="New thread">
            <Plus size={24} aria-hidden="true" />
          </button>
        </div>
      </header>

      <div className="thread-search" aria-hidden="true">Threads</div>

      <ThreadList
        activeThreadId={activeThreadId}
        selectedThreadId={selectedThreadId}
        threads={threads}
        onSelectThread={onSelectThread}
      />

      <footer className="storage-path">
        <Folder size={24} aria-hidden="true" />
        <span>{appInfo?.dataDir ?? "~/Library/Application Support/just-notes"}</span>
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
