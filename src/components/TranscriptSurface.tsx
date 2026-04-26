import { Waves } from "lucide-react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import { formatDuration } from "../lib/format";

type TranscriptSurfaceProps = {
  selectedThread: ThreadDetail | null;
};

export function TranscriptSurface({ selectedThread }: TranscriptSurfaceProps) {
  return (
    <section className="transcript-surface">
      {selectedThread && selectedThread.segments.length > 0 ? (
        selectedThread.segments.map((segment, index) => (
          <article key={`${segment.source}-${segment.startMs}-${segment.endMs}-${index}`} className="segment">
            <time>{formatDuration(segment.startMs)}</time>
            <strong>{segment.speaker}</strong>
            <p>{segment.text}</p>
          </article>
        ))
      ) : (
        <div className="empty-state">
          <Waves size={24} aria-hidden="true" />
          <span>{selectedThread ? "No transcript yet" : "Create or record a thread"}</span>
        </div>
      )}
    </section>
  );
}
