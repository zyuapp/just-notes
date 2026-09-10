import { Archive, Copy } from "lucide-react";
import { Button } from "./Button";

type TranscriptActionsProps = {
  canModify: boolean;
  onCopy: () => void;
  onArchive: () => void;
};

export function TranscriptActions({ canModify, onCopy, onArchive }: TranscriptActionsProps) {
  return (
    <div className="transcript-actions">
      <Button className="transcript-copy" onClick={onCopy}
        title="Copy transcript" aria-label="Copy transcript">
        <Copy size={14} aria-hidden="true" /><span>Copy</span>
      </Button>
      <Button variant="icon" onClick={onArchive} title="Archive thread"
        aria-label="Archive thread" disabled={!canModify}>
        <Archive size={15} aria-hidden="true" />
      </Button>
    </div>
  );
}
