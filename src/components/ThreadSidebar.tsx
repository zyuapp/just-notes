import { Plus, Search, Waves } from "lucide-react";
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
          <Waves size={18} aria-hidden="true" />
          <span>Just Notes</span>
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
