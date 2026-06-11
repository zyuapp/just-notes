import { AudioLines } from "lucide-react";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { formatDuration, formatThreadDate } from "../lib/format";
import { groupThreadsByDay } from "../lib/threads";

type ThreadListProps = {
  activeThreadId: string | null;
  selectedThreadId: string | null;
  threads: ThreadSummary[];
  searching: boolean;
  onSelectThread: (threadId: string) => void;
};

export function ThreadList({
  activeThreadId,
  selectedThreadId,
  threads,
  searching,
  onSelectThread,
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
};

function ThreadItem({ thread, active, selected, onSelect }: ThreadItemProps) {
  const dotClass =
    thread.status === "transcribing"
      ? "thread-dot transcribing"
      : active
        ? "thread-dot active"
        : "thread-dot";

  return (
    <button
      type="button"
      className={selected ? "thread-item selected" : "thread-item"}
      onClick={onSelect}
    >
      <span className={dotClass} />
      <span className="thread-title">{thread.title}</span>
      {thread.snippet && <span className="thread-snippet">{thread.snippet}</span>}
      <span className="thread-meta">
        {formatThreadDate(thread.updatedAtMs)}
        {thread.durationMs > 0 && <> · {formatDuration(thread.durationMs)}</>}
        {thread.status === "transcribing" ? (
          <> · Transcribing…</>
        ) : (
          <>
            {" "}
            · {thread.segmentCount} segment{thread.segmentCount === 1 ? "" : "s"}
          </>
        )}
        {thread.hasAudio && (
          <span className="thread-audio" title="Raw audio saved">
            <AudioLines size={13} aria-hidden="true" />
          </span>
        )}
      </span>
    </button>
  );
}
