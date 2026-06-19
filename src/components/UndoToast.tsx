import { useEffect } from "react";

const UNDO_TIMEOUT_MS = 6000;

type UndoToastProps = {
  notice: { threadId: string; title: string } | null;
  onUndo: () => void;
  onDismiss: () => void;
};

export function UndoToast({ notice, onUndo, onDismiss }: UndoToastProps) {
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(onDismiss, UNDO_TIMEOUT_MS);
    return () => clearTimeout(timer);
  }, [notice, onDismiss]);

  if (!notice) return null;

  return (
    <div className="undo-toast" role="status">
      <span className="undo-toast-text">
        Archived <strong>{notice.title || "Untitled thread"}</strong>
      </span>
      <button type="button" className="undo-toast-action" onClick={onUndo}>
        Undo
      </button>
    </div>
  );
}
