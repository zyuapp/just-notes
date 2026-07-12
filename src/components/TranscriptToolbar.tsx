import { Archive, Copy, RefreshCw, Search } from "lucide-react";

type TranscriptToolbarProps = {
  query: string;
  canModify: boolean;
  onQueryChange: (query: string) => void;
  onCopy: () => void;
  onArchive: () => void;
  // Present only when the thread has saved audio to re-transcribe.
  onReprocess?: () => void;
};

export function TranscriptToolbar({
  query,
  canModify,
  onQueryChange,
  onCopy,
  onArchive,
  onReprocess,
}: TranscriptToolbarProps) {
  return (
    <div className="transcript-toolbar">
      <label className="toolbar-search">
        <Search size={13} aria-hidden="true" />
        <input
          type="search"
          value={query}
          placeholder="Search transcript"
          onChange={(event) => onQueryChange(event.target.value)}
          aria-label="Search transcript"
        />
      </label>
      <div className="toolbar-actions">
        <button type="button" className="icon-button" onClick={onCopy} title="Copy transcript" aria-label="Copy transcript">
          <Copy size={15} aria-hidden="true" />
        </button>
        {onReprocess && (
          <button
            type="button"
            className="icon-button"
            onClick={onReprocess}
            title="Re-transcribe from saved audio"
            aria-label="Re-transcribe from saved audio"
            disabled={!canModify}
          >
            <RefreshCw size={15} aria-hidden="true" />
          </button>
        )}
        <button
          type="button"
          className="icon-button"
          onClick={onArchive}
          title="Archive thread"
          aria-label="Archive thread"
          disabled={!canModify}
        >
          <Archive size={15} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
