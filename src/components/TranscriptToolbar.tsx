import { Archive, Copy, FileDown, FolderOpen, Search, Users } from "lucide-react";
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
  onArchive: () => void;
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
  onArchive,
  onRenameSpeaker,
}: TranscriptToolbarProps) {
  const [showSpeakers, setShowSpeakers] = useState(false);

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
        <button type="button" className="icon-button" onClick={onExport} title="Export Markdown and reveal in Finder" aria-label="Export Markdown">
          <FileDown size={15} aria-hidden="true" />
        </button>
        <button type="button" className="icon-button" onClick={onReveal} title="Reveal files in Finder" aria-label="Reveal files in Finder">
          <FolderOpen size={15} aria-hidden="true" />
        </button>
        <button
          type="button"
          className={showSpeakers ? "icon-button active" : "icon-button"}
          onClick={() => setShowSpeakers((current) => !current)}
          title="Rename speakers"
          aria-label="Rename speakers"
          disabled={!canModify}
        >
          <Users size={15} aria-hidden="true" />
        </button>
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
      {showSpeakers && (
        <div className="speaker-renames">
          {speakers.map((speaker) => (
            <SpeakerRenameField
              key={`${speaker}:${speakerLabels[speaker] ?? ""}`}
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
