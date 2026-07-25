import { Archive, Copy } from "lucide-react";
import { Button } from "./Button";
import { SearchField } from "./SearchField";

type TranscriptToolbarProps = {
  query: string;
  canModify: boolean;
  onQueryChange: (query: string) => void;
  onCopy: () => void;
  onArchive: () => void;
};

export function TranscriptToolbar({
  query,
  canModify,
  onQueryChange,
  onCopy,
  onArchive,
}: TranscriptToolbarProps) {
  return (
    <div className="transcript-toolbar">
      <SearchField
        className="toolbar-search"
        value={query}
        placeholder="Search transcript"
        onChange={onQueryChange}
      />
      <div className="toolbar-actions">
        <Button variant="icon" onClick={onCopy} title="Copy transcript" aria-label="Copy transcript">
          <Copy size={15} aria-hidden="true" />
        </Button>
        <Button
          variant="icon"
          onClick={onArchive}
          title="Archive thread"
          aria-label="Archive thread"
          disabled={!canModify}
        >
          <Archive size={15} aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
