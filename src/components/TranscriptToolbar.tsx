import { Copy, FileDown, FolderOpen, Search, Trash2, Users } from "lucide-react";
import { useState } from "react";

type TranscriptToolbarProps = {
  query: string;
  speakers: string[];
  speakerLabels: Record<string, string>;
  canModify: boolean;
  onQueryChange: (query: string) => void;
  onCopy: () => void;
  onExport: () => void;
  onReveal: () => void;
  onDelete: () => void;
  onRenameSpeaker: (speaker: string, label: string) => void;
};

export function TranscriptToolbar({
  query,
  speakers,
  speakerLabels,
  canModify,
  onQueryChange,
  onCopy,
  onExport,
  onReveal,
  onDelete,
  onRenameSpeaker,
}: TranscriptToolbarProps) {
  const [showSpeakers, setShowSpeakers] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  return (
    <div className="transcript-toolbar">
      <div className="toolbar-search">
        <Search size={16} aria-hidden="true" />
        <input
          type="search"
          value={query}
          placeholder="Search transcript"
          onChange={(event) => onQueryChange(event.target.value)}
          aria-label="Search transcript"
        />
      </div>
      <div className="toolbar-actions">
        <button type="button" onClick={onCopy} title="Copy transcript">
          <Copy size={17} aria-hidden="true" />
          <span>Copy</span>
        </button>
        <button type="button" onClick={onExport} title="Export Markdown and reveal in Finder">
          <FileDown size={17} aria-hidden="true" />
          <span>Export</span>
        </button>
        <button type="button" onClick={onReveal} title="Reveal files in Finder">
          <FolderOpen size={17} aria-hidden="true" />
          <span>Files</span>
        </button>
        <button
          type="button"
          onClick={() => setShowSpeakers((current) => !current)}
          title="Rename speakers"
          className={showSpeakers ? "active" : ""}
          disabled={!canModify}
        >
          <Users size={17} aria-hidden="true" />
          <span>Speakers</span>
        </button>
        <button
          type="button"
          className="danger"
          onClick={() => {
            if (confirmDelete) {
              setConfirmDelete(false);
              onDelete();
            } else {
              setConfirmDelete(true);
            }
          }}
          onBlur={() => setConfirmDelete(false)}
          title="Delete thread"
          disabled={!canModify}
        >
          <Trash2 size={17} aria-hidden="true" />
          <span>{confirmDelete ? "Confirm" : "Delete"}</span>
        </button>
      </div>
      {showSpeakers && (
        <div className="speaker-renames">
          {speakers.map((speaker) => (
            <SpeakerRenameField
              key={speaker}
              speaker={speaker}
              label={speakerLabels[speaker] ?? ""}
              onRename={onRenameSpeaker}
            />
          ))}
        </div>
      )}
    </div>
  );
}

type SpeakerRenameFieldProps = {
  speaker: string;
  label: string;
  onRename: (speaker: string, label: string) => void;
};

function SpeakerRenameField({ speaker, label, onRename }: SpeakerRenameFieldProps) {
  const [draft, setDraft] = useState(label);

  const commit = () => {
    if (draft.trim() !== label) {
      onRename(speaker, draft);
    }
  };

  return (
    <label className="speaker-rename">
      <span>{speaker} →</span>
      <input
        value={draft}
        placeholder={speaker}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit();
        }}
      />
    </label>
  );
}
