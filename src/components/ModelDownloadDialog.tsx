import { Download } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef } from "react";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import { downloadActionLabel, downloadConfirmationMessage } from "../lib/transcriptionModel";
import { useDismissOnEscape } from "./useDismissOnEscape";

type ModelDownloadDialogProps = {
  model: TranscriptionModelStatus;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ModelDownloadDialog({ model, onCancel, onConfirm }: ModelDownloadDialogProps) {
  useDismissOnEscape(onCancel);
  const returnFocusRef = useRef(
    document.activeElement instanceof HTMLElement ? document.activeElement : null,
  );

  useEffect(() => {
    const returnFocus = returnFocusRef.current;
    return () => returnFocus?.focus();
  }, []);

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab") return;

    const actions = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"),
    );
    const first = actions[0];
    const last = actions[actions.length - 1];
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      className="model-download-dialog-overlay"
      onKeyDown={handleKeyDown}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <section
        className="model-download-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="model-download-dialog-title"
        aria-describedby="model-download-dialog-description"
      >
        <div className="model-download-dialog-icon" aria-hidden="true">
          <Download size={18} />
        </div>
        <div className="model-download-dialog-copy">
          <h2 id="model-download-dialog-title">Download local transcription model?</h2>
          <p id="model-download-dialog-description">{downloadConfirmationMessage(model)}</p>
        </div>
        <div className="model-download-dialog-actions">
          <button type="button" className="model-download-dialog-cancel" onClick={onCancel}>
            Not now
          </button>
          <button
            type="button"
            className="model-download-dialog-confirm"
            onClick={onConfirm}
            autoFocus
          >
            {downloadActionLabel(model)}
          </button>
        </div>
      </section>
    </div>
  );
}
