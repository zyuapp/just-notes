import { Download } from "lucide-react";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import { downloadActionLabel, downloadConfirmationMessage } from "../lib/transcriptionModel";
import { useDismissOnEscape } from "./useDismissOnEscape";
import { useModalFocus } from "./useModalFocus";

type ModelDownloadDialogProps = {
  model: TranscriptionModelStatus;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ModelDownloadDialog({ model, onCancel, onConfirm }: ModelDownloadDialogProps) {
  useDismissOnEscape(onCancel);
  const handleKeyDown = useModalFocus();

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
