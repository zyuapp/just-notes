import { Archive, ArchiveRestore, ArrowLeft, Trash2 } from "lucide-react";
import { useState } from "react";
import type { ThreadSummary } from "../bindings/ThreadSummary";
import { formatThreadDate } from "../lib/format";
import { Button } from "./Button";
import { FullscreenView } from "./FullscreenView";
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
    <FullscreenView
      ariaLabel="Archive"
      navigation={<ArchiveNavigation onClose={onClose} />}
      title="Archive"
      description="Restore recordings to your library or permanently delete the ones you no longer need."
      onClose={onClose}
    >
      {error && <p className="archive-error">{error}</p>}

      {items === null ? (
        <p className="archive-empty">Loading…</p>
      ) : items.length === 0 ? (
        <p className="archive-empty">
          No archived recordings. Recordings you archive will appear here until you restore or
          permanently delete them.
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
    </FullscreenView>
  );
}

function ArchiveNavigation({ onClose }: { onClose: () => void }) {
  return (
    <nav className="archive-nav fullscreen-nav" aria-label="Archive navigation">
      <button
        type="button"
        className="archive-nav-item fullscreen-nav-item fullscreen-nav-back"
        onClick={onClose}
        aria-label="Back to notes"
        title="Back to notes"
      >
        <ArrowLeft size={16} aria-hidden="true" />
      </button>
      <div className="archive-nav-title fullscreen-nav-title">Archive</div>
      <div className="archive-nav-item fullscreen-nav-item active">
        <Archive size={16} aria-hidden="true" />
        <span>Recordings</span>
      </div>
    </nav>
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
        <Button
          size="compact"
          leadingIcon={<ArchiveRestore size={14} />}
          onClick={onRestore}
          disabled={disabled}
        >
          Restore
        </Button>
        <Button
          size="compact"
          variant="danger"
          leadingIcon={<Trash2 size={14} />}
          className={confirmDelete.armed ? "armed" : undefined}
          onClick={confirmDelete.trigger}
          onBlur={confirmDelete.reset}
          disabled={disabled}
        >
          {confirmDelete.armed ? "Confirm delete" : "Delete permanently"}
        </Button>
      </div>
    </li>
  );
}
