import { Download } from "lucide-react";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import { downloadActionLabel, downloadConfirmationMessage } from "../lib/transcriptionModel";
import { Button } from "./Button";
import { ModalFrame } from "./ModalFrame";

type ModelDownloadDialogProps = {
  model: TranscriptionModelStatus;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ModelDownloadDialog({ model, onCancel, onConfirm }: ModelDownloadDialogProps) {
  return (
    <ModalFrame
      role="alertdialog"
      labelledBy="model-download-dialog-title"
      describedBy="model-download-dialog-description"
      icon={<Download size={18} />}
      onDismiss={onCancel}
      closeOnBackdrop
      actions={
        <>
          <Button onClick={onCancel}>Not now</Button>
          <Button variant="primary" onClick={onConfirm} autoFocus>
            {downloadActionLabel(model)}
          </Button>
        </>
      }
    >
      <h2 id="model-download-dialog-title">Download local transcription model?</h2>
      <p id="model-download-dialog-description">{downloadConfirmationMessage(model)}</p>
    </ModalFrame>
  );
}
