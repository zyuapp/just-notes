import { ArchiveRestore, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { formatThreadDate } from "../lib/format";
import { useConfirmAction } from "./useConfirmAction";

type ArchivedViewProps = {
  items: ThreadSummary[] | null;
  error: string | null;
  onReload: () => Promise<void>;
  onClose: () => void;
  onRestore: (threadId: string) => Promise<void>;
  onDeletePermanently: (threadId: string) => Promise<void>;
};

export function ArchivedView({
  items,
  error,
  onReload,
  onClose,
  onRestore,
  onDeletePermanently,
}: ArchivedViewProps) {
  const [busy, setBusy] = useState(false);

  // macOS webviews never deliver keydown for Escape (tauri#5790); keyup does.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("keyup", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("keyup", onKey);
    };
  }, [onClose]);

  // Reload after a mutation so the list reflects what actually happened, even
  // when the action reported an error through the app toast. The busy flag
  // serializes actions so a double-click can't fire a second restore against an
  // already-moved thread.
  const runAction = async (action: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    try {
      await action();
      await onReload();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="archive-overlay" role="dialog" aria-modal="true" aria-label="Archived recordings">
      <div className="archive-panel">
        <header>
          <h2>Archived</h2>
          <button
            type="button"
            className="icon-button"
            onClick={onClose}
            aria-label="Close archive"
          >
            <X size={15} aria-hidden="true" />
          </button>
        </header>

        {error && <p className="archive-error">{error}</p>}

        {items === null ? (
          <p className="archive-empty">Loading…</p>
        ) : items.length === 0 ? (
          <p className="archive-empty">
            No archived recordings. Archived threads are kept out of your transcripts folder until
            you restore or permanently delete them.
          </p>
        ) : (
          <ul className="archive-list">
            {items.map((item) => (
              <ArchivedRow
                key={item.id}
                thread={item}
                disabled={busy}
                onRestore={() => void runAction(() => onRestore(item.id))}
                onDelete={() => void runAction(() => onDeletePermanently(item.id))}
              />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

type ArchivedRowProps = {
  thread: ThreadSummary;
  disabled: boolean;
  onRestore: () => void;
  onDelete: () => void;
};

function ArchivedRow({ thread, disabled, onRestore, onDelete }: ArchivedRowProps) {
  const confirmDelete = useConfirmAction(onDelete);

  return (
    <li className="archive-row">
      <div className="archive-row-main">
        <span className="archive-row-title">{thread.title || "Untitled thread"}</span>
        <span className="archive-row-meta">{formatThreadDate(thread.createdAtMs)}</span>
      </div>
      <div className="archive-row-actions">
        <button type="button" className="archive-restore" onClick={onRestore} disabled={disabled}>
          <ArchiveRestore size={14} aria-hidden="true" />
          <span>Restore</span>
        </button>
        <button
          type="button"
          className={confirmDelete.armed ? "archive-delete armed" : "archive-delete"}
          onClick={confirmDelete.trigger}
          onBlur={confirmDelete.reset}
          disabled={disabled}
        >
          <Trash2 size={14} aria-hidden="true" />
          <span>{confirmDelete.armed ? "Confirm delete" : "Delete permanently"}</span>
        </button>
      </div>
    </li>
  );
}
