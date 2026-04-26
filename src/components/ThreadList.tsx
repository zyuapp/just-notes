import type { ThreadSummary } from "../bindings/ThreadSummary";
import { formatThreadDate } from "../lib/format";

type ThreadListProps = {
  activeThreadId: string | null;
  selectedThreadId: string | null;
  threads: ThreadSummary[];
  onSelectThread: (threadId: string) => void;
};

export function ThreadList({
  activeThreadId,
  selectedThreadId,
  threads,
  onSelectThread,
}: ThreadListProps) {
  return (
    <nav className="thread-list" aria-label="Saved threads">
      {threads.map((thread) => (
        <button
          key={thread.id}
          type="button"
          className={thread.id === selectedThreadId ? "thread-item selected" : "thread-item"}
          onClick={() => onSelectThread(thread.id)}
        >
          <span className={thread.id === activeThreadId ? "thread-dot active" : "thread-dot"} />
          <span className="thread-title">{thread.title}</span>
          <span className="thread-meta">
            {formatThreadDate(thread.updatedAtMs)} · {thread.segmentCount}
          </span>
        </button>
      ))}
    </nav>
  );
}
