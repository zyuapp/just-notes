import { SearchField } from "./SearchField";

type TranscriptToolbarProps = {
  query: string;
  onQueryChange: (query: string) => void;
};

export function TranscriptToolbar({
  query,
  onQueryChange,
}: TranscriptToolbarProps) {
  return (
    <div className="transcript-toolbar">
      <span className="toolbar-label">Transcript</span>
      <SearchField
        className="toolbar-search"
        value={query}
        placeholder="Find in transcript"
        onChange={onQueryChange}
      />
    </div>
  );
}
