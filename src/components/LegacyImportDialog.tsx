import { Check, FolderInput } from "lucide-react";
import type { ReactNode } from "react";
import type { LegacyImportViewState } from "../features/app/legacyImportState";
import { Button } from "./Button";
import {
  completeImportSummary,
  formatImportBytes,
  importActionLabel,
} from "./legacyImportPresentation";
import { ModalFrame } from "./ModalFrame";

export type LegacyImportDialogProps = {
  state: LegacyImportViewState;
  onClose: () => void;
  onLocate: () => void;
  onConfirm: () => void;
  onViewImported: () => void;
};

export function LegacyImportDialog(props: LegacyImportDialogProps) {
  const busy = props.state.stage === "locating" || props.state.stage === "importing";
  const onDismiss = busy ? undefined : props.onClose;
  return props.state.stage === "complete" ? (
    <CompleteStep {...props} onDismiss={onDismiss} />
  ) : props.state.stage === "preview" || props.state.stage === "importing" ? (
    <PreviewStep {...props} onDismiss={onDismiss} />
  ) : (
    <LocateStep {...props} onDismiss={onDismiss} />
  );
}

function ImportFrame(props: {
  icon: ReactNode;
  success?: boolean;
  children: ReactNode;
  actions: ReactNode;
  onDismiss?: () => void;
}) {
  return (
    <ModalFrame
      className="modal-frame-wide"
      labelledBy="legacy-import-title"
      describedBy="legacy-import-description"
      icon={props.icon}
      iconTone={props.success ? "success" : "default"}
      onDismiss={props.onDismiss}
      actions={props.actions}
    >
      {props.children}
    </ModalFrame>
  );
}

function LocateStep(props: LegacyImportDialogProps & { onDismiss?: () => void }) {
  const locating = props.state.stage === "locating";
  return (
    <ImportFrame
      icon={<FolderInput size={19} />}
      onDismiss={props.onDismiss}
      actions={
        <>
          <Button onClick={props.onClose} disabled={locating}>Cancel</Button>
          <Button variant="primary" onClick={props.onLocate} disabled={locating} autoFocus>
            {locating ? "Waiting for macOS…" : "Locate previous data"}
          </Button>
        </>
      }
    >
      <h2 id="legacy-import-title">Import previous recordings</h2>
      <p id="legacy-import-description">
        Merge recordings from the earlier version without changing current recordings, settings,
        or the downloaded transcription model.
      </p>
      <p className="legacy-import-detail">
        macOS will ask for access to the hidden <code>.just-notes</code> folder. The picker opens in
        your home folder with hidden files visible.
      </p>
      {props.state.stage === "intro" && props.state.error && (
        <p className="legacy-import-error" role="alert">{props.state.error}</p>
      )}
    </ImportFrame>
  );
}

function PreviewStep(props: LegacyImportDialogProps & { onDismiss?: () => void }) {
  if (props.state.stage !== "preview" && props.state.stage !== "importing") return null;
  const importing = props.state.stage === "importing";
  const { preview } = props.state;
  const importCount = preview.activeRecordings + preview.archivedRecordings;
  return (
    <ImportFrame
      icon={<FolderInput size={19} />}
      onDismiss={props.onDismiss}
      actions={
        <>
          <Button onClick={props.onClose} disabled={importing}>Cancel</Button>
          <Button variant="primary" onClick={props.onConfirm} disabled={importing} autoFocus>
            {importActionLabel(importing, importCount)}
          </Button>
        </>
      }
    >
      <h2 id="legacy-import-title">Review the import</h2>
      <p id="legacy-import-description">Nothing has been changed yet.</p>
      <dl className="legacy-import-summary">
        <SummaryRow label="Active recordings" value={preview.activeRecordings} />
        <SummaryRow label="Archived recordings" value={preview.archivedRecordings} />
        <SummaryRow label="Exact duplicates skipped" value={preview.duplicates} />
        <SummaryRow label="Conflicts preserved as copies" value={preview.conflicts} />
        <SummaryRow label="Data to copy" value={formatImportBytes(preview.bytesToCopy)} />
      </dl>
      <p className="legacy-import-source" title={preview.sourcePath}>{preview.sourcePath}</p>
    </ImportFrame>
  );
}

function CompleteStep(props: LegacyImportDialogProps & { onDismiss?: () => void }) {
  if (props.state.stage !== "complete") return null;
  const { result } = props.state;
  return (
    <ImportFrame
      icon={<Check size={20} />}
      success
      onDismiss={props.onDismiss}
      actions={
        <>
          <Button onClick={props.onClose}>Done</Button>
          <Button variant="primary" onClick={props.onViewImported} autoFocus>
            View recordings
          </Button>
        </>
      }
    >
      <h2 id="legacy-import-title">Import complete</h2>
      <p id="legacy-import-description">{completeImportSummary(result)}</p>
      <p className="legacy-import-detail">You can run this import again; exact duplicates will be skipped.</p>
    </ImportFrame>
  );
}

function SummaryRow({ label, value }: { label: string; value: string | number }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}
