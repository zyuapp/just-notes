import { Circle, FilePlus2, Waves } from "lucide-react";
import type { ThreadDetail } from "../bindings/ThreadDetail";
import type { RecorderState } from "../features/app/state";
import { formatDuration } from "../lib/format";

type TranscriptSurfaceProps = {
  recorderState: RecorderState;
  selectedThread: ThreadDetail | null;
  onCreateThread: () => void;
  onStartRecording: () => void;
};

export function TranscriptSurface({
  recorderState,
  selectedThread,
  onCreateThread,
  onStartRecording,
}: TranscriptSurfaceProps) {
  const canStart = recorderState === "idle";

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
          <div className="empty-mark">
            <Waves size={28} aria-hidden="true" />
          </div>
          <div>
            <h2>{selectedThread ? "Ready when you are" : "Ready to capture"}</h2>
            <p>
              {selectedThread
                ? "Start recording to add the first transcript segment to this thread."
                : "Record a conversation now, or create a blank thread for notes you will fill in later."}
            </p>
          </div>
          <div className="empty-actions">
            <button
              type="button"
              className="empty-primary"
              onClick={onStartRecording}
              disabled={!canStart}
            >
              <Circle size={15} aria-hidden="true" />
              <span>Start recording</span>
            </button>
            {!selectedThread && (
              <button type="button" className="empty-secondary" onClick={onCreateThread}>
                <FilePlus2 size={15} aria-hidden="true" />
                <span>New blank thread</span>
              </button>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
