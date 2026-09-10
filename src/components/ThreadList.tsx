import { AudioLines } from "lucide-react";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { formatCompactDuration, formatTimeOfDay } from "../lib/format";
import { groupThreadsByDay } from "../lib/threads";

type ThreadListProps = {
  activeThreadId: string | null;
  selectedThreadId: string | null;
  threads: ThreadSummary[];
  searching: boolean;
  onSelectThread: (threadId: string) => void;
  onOpenMenu: (thread: ThreadSummary, x: number, y: number) => void;
};

export function ThreadList({
  activeThreadId,
  selectedThreadId,
  threads,
  searching,
  onSelectThread,
  onOpenMenu,
}: ThreadListProps) {
  if (threads.length === 0) {
    return (
      <nav className="thread-list" aria-label="Saved threads">
        <p className="thread-list-empty">
          {searching ? "No recordings match this search." : "No recordings yet."}
        </p>
      </nav>
    );
  }

  return (
    <nav className="thread-list" aria-label="Saved threads">
      {groupThreadsByDay(threads).map((group) => (
        <section key={group.label} className="thread-group">
          <h2 className="thread-group-label">{group.label}</h2>
          {group.threads.map((thread) => (
            <ThreadItem
              key={thread.id}
              thread={thread}
              active={thread.id === activeThreadId}
              selected={thread.id === selectedThreadId}
              onSelect={() => onSelectThread(thread.id)}
              onOpenMenu={(x, y) => onOpenMenu(thread, x, y)}
            />
          ))}
        </section>
      ))}
    </nav>
  );
}

type ThreadItemProps = {
  thread: ThreadSummary;
  active: boolean;
  selected: boolean;
  onSelect: () => void;
  onOpenMenu: (x: number, y: number) => void;
};

function ThreadItem({ thread, active, selected, onSelect, onOpenMenu }: ThreadItemProps) {
  const transcribing = thread.status === "transcribing";
  const segmentCount = `${thread.segmentCount} segment${thread.segmentCount === 1 ? "" : "s"}`;

  return (
    <button
      type="button"
      className={selected ? "thread-item selected" : "thread-item"}
      aria-current={selected ? "page" : undefined}
      data-thread-id={thread.id}
      onClick={onSelect}
      onContextMenu={(event) => {
        event.preventDefault();
        onOpenMenu(event.clientX, event.clientY);
      }}
    >
      <span className="thread-row">
        {(active || transcribing) && (
          <span className={active ? "thread-dot live" : "thread-dot transcribing"} />
        )}
        <span className="thread-title">{thread.title}</span>
      </span>
      {thread.snippet && <span className="thread-snippet">{thread.snippet}</span>}
      <span className="thread-meta" title={segmentCount}>
        {thread.hasAudio && (
          <span className="thread-audio" title="Raw audio saved">
            <AudioLines size={11} aria-hidden="true" />
          </span>
        )}
        <span>{thread.durationMs > 0 ? formatCompactDuration(thread.durationMs) : segmentCount}</span>
        <span aria-hidden="true">·</span>
        <time className="thread-time">{formatTimeOfDay(thread.updatedAtMs)}</time>
        {transcribing && <span>· Transcribing…</span>}
      </span>
    </button>
  );
}
