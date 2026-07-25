import { Pencil } from "lucide-react";
import { useState } from "react";
import type { TranscriptSegment } from "../bindings/TranscriptSegment";
import { formatDuration } from "../lib/format";
import { segmentClasses, type SpeakerRunEdges } from "../lib/transcript";

type SegmentBlockProps = {
  segment: TranscriptSegment;
  index: number;
  runEdges: SpeakerRunEdges;
  active: boolean;
  editable: boolean;
  onSaveText: (index: number, text: string) => void;
};

export function SegmentBlock({
  segment,
  index,
  runEdges,
  active,
  editable,
  onSaveText,
}: SegmentBlockProps) {
  const [draft, setDraft] = useState<string | null>(null);

  const commit = () => {
    if (draft !== null && draft.trim() && draft.trim() !== segment.text) {
      onSaveText(index, draft.trim());
    }
    setDraft(null);
  };

  return (
    <article className={segmentClasses(segment, runEdges, active)}>
      {runEdges.isStart && (
        <header className="segment-head">
          <strong>{segment.speaker}</strong>
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
