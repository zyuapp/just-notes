import { Check, FolderInput } from "lucide-react";
import type { LegacyImportViewState } from "../features/app/legacyImportState";
import { useDismissOnEscape } from "./useDismissOnEscape";
import { useModalFocus } from "./useModalFocus";

export type LegacyImportDialogProps = {
  state: LegacyImportViewState;
  onClose: () => void;
  onLocate: () => void;
  onConfirm: () => void;
  onViewImported: () => void;
};

export function LegacyImportDialog(props: LegacyImportDialogProps) {
  const busy = props.state.stage === "locating" || props.state.stage === "importing";
  useDismissOnEscape(() => {
    if (!busy) props.onClose();
  });
  const handleKeyDown = useModalFocus();

  return (
    <div className="legacy-import-overlay" onKeyDown={handleKeyDown}>
      <section
        className="legacy-import-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="legacy-import-title"
        aria-describedby="legacy-import-description"
      >
        {props.state.stage === "complete" ? (
          <CompleteStep {...props} />
        ) : props.state.stage === "preview" || props.state.stage === "importing" ? (
          <PreviewStep {...props} />
        ) : (
          <LocateStep {...props} />
        )}
      </section>
    </div>
  );
}

function LocateStep(props: LegacyImportDialogProps) {
  const locating = props.state.stage === "locating";
  return <>
    <DialogIcon><FolderInput size={19} /></DialogIcon>
    <div className="legacy-import-copy">
      <h2 id="legacy-import-title">Import previous recordings</h2>
      <p id="legacy-import-description">
        Just Notes will merge recordings from the earlier version. Current recordings, settings,
        and the downloaded transcription model stay unchanged.
      </p>
      <p className="legacy-import-detail">
        macOS will ask you to grant access to the hidden <code>.just-notes</code> folder. The picker
        opens in your home folder with hidden files visible.
      </p>
      {props.state.stage === "intro" && props.state.error && (
        <p className="legacy-import-error" role="alert">{props.state.error}</p>
      )}
    </div>
    <DialogActions>
      <button type="button" className="legacy-import-secondary" onClick={props.onClose} disabled={locating}>
        Cancel
      </button>
      <button type="button" className="legacy-import-primary" onClick={props.onLocate} disabled={locating} autoFocus>
        {locating ? "Waiting for macOS…" : "Locate previous data"}
      </button>
    </DialogActions>
  </>;
}

function PreviewStep(props: LegacyImportDialogProps) {
  if (props.state.stage !== "preview" && props.state.stage !== "importing") return null;
  const { preview } = props.state;
  const importing = props.state.stage === "importing";
  const importCount = preview.activeRecordings + preview.archivedRecordings;
  return <>
    <DialogIcon><FolderInput size={19} /></DialogIcon>
    <div className="legacy-import-copy">
      <h2 id="legacy-import-title">Review the import</h2>
      <p id="legacy-import-description">Nothing has been changed yet.</p>
      <dl className="legacy-import-summary">
        <SummaryRow label="Active recordings" value={preview.activeRecordings} />
        <SummaryRow label="Archived recordings" value={preview.archivedRecordings} />
        <SummaryRow label="Exact duplicates skipped" value={preview.duplicates} />
        <SummaryRow label="Conflicts preserved as copies" value={preview.conflicts} />
        <SummaryRow label="Data to copy" value={formatBytes(preview.bytesToCopy)} />
      </dl>
      <p className="legacy-import-source" title={preview.sourcePath}>{preview.sourcePath}</p>
    </div>
    <DialogActions>
      <button type="button" className="legacy-import-secondary" onClick={props.onClose} disabled={importing}>
        Cancel
      </button>
      <button type="button" className="legacy-import-primary" onClick={props.onConfirm} disabled={importing} autoFocus>
        {importing
          ? "Importing…"
          : importCount === 0
            ? "Finish import"
            : `Import ${importCount} recording${importCount === 1 ? "" : "s"}`}
      </button>
    </DialogActions>
  </>;
}

function CompleteStep(props: LegacyImportDialogProps) {
  if (props.state.stage !== "complete") return null;
  const { result } = props.state;
  return <>
    <DialogIcon success><Check size={20} /></DialogIcon>
    <div className="legacy-import-copy">
      <h2 id="legacy-import-title">Import complete</h2>
      <p id="legacy-import-description">
        {result.imported} recording{result.imported === 1 ? "" : "s"} imported
        {result.duplicates > 0 ? ` · ${result.duplicates} duplicate${result.duplicates === 1 ? "" : "s"} skipped` : ""}
        {result.conflicts > 0 ? ` · ${result.conflicts} conflict${result.conflicts === 1 ? "" : "s"} preserved` : ""}
      </p>
      <p className="legacy-import-detail">You can safely run this import again later; exact duplicates will be skipped.</p>
    </div>
    <DialogActions>
      <button type="button" className="legacy-import-secondary" onClick={props.onClose}>Done</button>
      <button type="button" className="legacy-import-primary" onClick={props.onViewImported} autoFocus>
        View recordings
      </button>
    </DialogActions>
  </>;
}

function DialogIcon({ children, success = false }: { children: React.ReactNode; success?: boolean }) {
  return <div className={success ? "legacy-import-icon success" : "legacy-import-icon"} aria-hidden="true">{children}</div>;
}

function DialogActions({ children }: { children: React.ReactNode }) {
  return <div className="legacy-import-actions">{children}</div>;
}

function SummaryRow({ label, value }: { label: string; value: string | number }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${Math.round(bytes / (1024 * 1024))} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}
