import { Pencil } from "lucide-react";
import { useState } from "react";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import { formatDuration } from "../lib/format";

type SegmentBlockProps = {
  segment: TranscriptSegment;
  index: number;
  showHeader: boolean;
  speakerLabel: string;
  active: boolean;
  editable: boolean;
  onSaveText: (index: number, text: string) => void;
};

export function SegmentBlock({
  segment,
  index,
  showHeader,
  speakerLabel,
  active,
  editable,
  onSaveText,
}: SegmentBlockProps) {
  const [draft, setDraft] = useState<string | null>(null);

  const classes = ["segment"];
  if (segment.source === "mic") classes.push("you");
  if (!showHeader) classes.push("continuation");
  if (active) classes.push("active");

  const commit = () => {
    if (draft !== null && draft.trim() && draft.trim() !== segment.text) {
      onSaveText(index, draft.trim());
    }
    setDraft(null);
  };

  return (
    <article className={classes.join(" ")}>
      {showHeader && (
        <header className="segment-head">
          <strong>{speakerLabel}</strong>
          <time>{formatDuration(segment.startMs)}</time>
        </header>
      )}
      {draft === null ? (
        <p>{segment.text}</p>
      ) : (
        <textarea
          className="segment-editor"
          value={draft}
          autoFocus
          rows={Math.max(2, Math.ceil(draft.length / 80))}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              commit();
            }
            if (event.key === "Escape") setDraft(null);
          }}
          // macOS webviews never deliver keydown for Escape (tauri#5790); keyup does.
          onKeyUp={(event) => {
            if (event.key === "Escape") setDraft(null);
          }}
          aria-label="Edit transcript text"
        />
      )}
      {editable && draft === null && (
        <button
          type="button"
          className="segment-edit"
          aria-label="Edit segment"
          onClick={() => setDraft(segment.text)}
        >
          <Pencil size={13} aria-hidden="true" />
        </button>
      )}
    </article>
  );
}
