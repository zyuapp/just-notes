import { Plus, Search } from "lucide-react";
import type { AppInfo } from "../bindings/AppInfo";
import type { MeterPayload } from "../bindings/MeterPayload";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import type { RecorderState } from "../features/app/state";
import { Meter } from "./Meter";
import { RecorderControls } from "./RecorderControls";
import { ThreadList } from "./ThreadList";

type ThreadSidebarProps = {
  activeThreadId: string | null;
  appInfo: AppInfo | null;
  meters: MeterPayload;
  recorderState: RecorderState;
  selectedThreadId: string | null;
  statusLabel: string;
  threads: ThreadSummary[];
  onCreateThread: () => void;
  onSelectThread: (threadId: string) => void;
  onStartFixtureRecording: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
};

export function ThreadSidebar({
  activeThreadId,
  appInfo,
  meters,
  recorderState,
  selectedThreadId,
  statusLabel,
  threads,
  onCreateThread,
  onSelectThread,
  onStartFixtureRecording,
  onStartRecording,
  onStopRecording,
}: ThreadSidebarProps) {
  return (
    <aside className="thread-sidebar" aria-label="Threads">
      <header className="sidebar-head">
        <div className="brand">
          <BrandMark />
        </div>
        <button type="button" className="icon-button" onClick={onCreateThread} aria-label="New thread">
          <Plus size={18} aria-hidden="true" />
        </button>
      </header>

      <RecorderControls
        fixtureMode={appInfo?.fixtureMode ?? false}
        recorderState={recorderState}
        statusLabel={statusLabel}
        onStart={onStartRecording}
        onStartFixture={onStartFixtureRecording}
        onStop={onStopRecording}
      />

      <div className="meters">
        <Meter label="Mic" source="mic" level={meters.micLevel} />
        <Meter label="System" source="system" level={meters.systemLevel} />
      </div>

      <div className="thread-search" aria-hidden="true">
        <Search size={14} />
        <span>Threads</span>
      </div>

      <ThreadList
        activeThreadId={activeThreadId}
        selectedThreadId={selectedThreadId}
        threads={threads}
        onSelectThread={onSelectThread}
      />

      <footer className="storage-path">{appInfo?.dataDir ?? "~/.just-notes"}</footer>
    </aside>
  );
}

function BrandMark() {
  return (
    <svg className="brand-mark" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id="brand-mark-gradient" x1="6" y1="2" x2="18" y2="22" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#b8b4ff" />
          <stop offset="0.46" stopColor="#637eff" />
          <stop offset="1" stopColor="#1746f5" />
        </linearGradient>
      </defs>
      <path
        className="brand-mark-shape"
        d="M6.3 4.5h10.1c.7 0 1.3.2 1.8.7l1.1 1.1c.5.5.8 1.2.8 1.9v7.3c0 2.2-1.8 4-4 4H9.2l-3.1 2.1c-.8.5-1.8 0-1.8-.9V6.5c0-1.1.9-2 2-2Z"
      />
      <path className="brand-mark-line" d="M8 9.1h8" />
      <path className="brand-mark-line" d="M8 12h6.8" />
      <path className="brand-mark-wave" d="M7.9 15.4h2.2l.8-1.2 1.3 3 1.4-4.2 1.3 2.4h2" />
    </svg>
  );
}
