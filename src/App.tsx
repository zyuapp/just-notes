import type { MeterPayload } from "./bindings/MeterPayload";
import { useJustNotesController } from "./features/app/useJustNotesController";
import { useLiveTranscriptEvents } from "./features/app/useLiveTranscriptEvents";
import {
  appReducer,
  getActiveThreadId,
  getStatusLabel,
  initialAppState,
} from "./features/app/state";
import { formatDuration, formatThreadDate } from "./lib/format";
import {
  FlaskConical,
  Circle,
  FileText,
  Mic,
  Plus,
  Radio,
  Search,
  Square,
  Waves,
} from "lucide-react";
import { useMemo, useReducer } from "react";

const meterBars = Array.from({ length: 18 }, (_, index) => index);

function Meter({ label, level, source }: { label: string; level: number; source: "mic" | "system" }) {
  const activeBars = Math.round(Math.min(1, Math.max(0, level)) * meterBars.length);
  const Icon = source === "mic" ? Mic : Radio;

  return (
    <section className="meter" aria-label={`${label} level`}>
      <div className="meter-heading">
        <Icon size={15} aria-hidden="true" />
        <span>{label}</span>
        <strong>{Math.round(level * 100)}%</strong>
      </div>
      <div className="meter-grid" aria-hidden="true">
        {meterBars.map((bar) => (
          <span key={bar} className={bar < activeBars ? "active" : ""} />
        ))}
      </div>
    </section>
  );
}

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const actions = useJustNotesController(state, dispatch);
  useLiveTranscriptEvents(dispatch);

  const activeThreadId = getActiveThreadId(state);
  const canStart = state.recorderState === "idle";
  const statusLabel = useMemo(() => getStatusLabel(state), [state]);

  return (
    <main className="app-shell">
      <aside className="thread-sidebar" aria-label="Threads">
        <header className="sidebar-head">
          <div className="brand">
            <Waves size={18} aria-hidden="true" />
            <span>Just Notes</span>
          </div>
          <button type="button" className="icon-button" onClick={actions.createThread} aria-label="New thread">
            <Plus size={18} aria-hidden="true" />
          </button>
        </header>

        <div className="sidebar-actions">
          <button
            type="button"
            className={state.recorderState === "recording" ? "record-button recording" : "record-button"}
            onClick={state.recorderState === "recording" ? actions.stopRecording : actions.startRecording}
            disabled={state.recorderState === "starting" || state.recorderState === "stopping"}
            aria-label={state.recorderState === "recording" ? "Stop recording" : "Start recording"}
          >
            {state.recorderState === "recording" ? (
              <Square size={16} aria-hidden="true" />
            ) : (
              <Circle size={16} aria-hidden="true" />
            )}
            <span>{state.recorderState === "recording" ? "Stop" : "Record"}</span>
          </button>
          <div className="capture-state">{statusLabel}</div>
          {state.appInfo?.fixtureMode && (
            <button
              type="button"
              className="fixture-button"
              onClick={actions.startFixtureRecording}
              disabled={!canStart}
              aria-label="Start QA fixture recording"
            >
              <FlaskConical size={15} aria-hidden="true" />
              <span>QA fixture</span>
            </button>
          )}
        </div>

        <div className="meters">
          <Meter label="Mic" source="mic" level={state.meters.micLevel} />
          <Meter label="System" source="system" level={state.meters.systemLevel} />
        </div>

        <div className="thread-search" aria-hidden="true">
          <Search size={14} />
          <span>Threads</span>
        </div>

        <nav className="thread-list" aria-label="Saved threads">
          {state.threads.map((thread) => (
            <button
              key={thread.id}
              type="button"
              className={thread.id === state.selectedThreadId ? "thread-item selected" : "thread-item"}
              onClick={() => void actions.selectThread(thread.id)}
            >
              <span className={thread.id === activeThreadId ? "thread-dot active" : "thread-dot"} />
              <span className="thread-title">{thread.title}</span>
              <span className="thread-meta">
                {formatThreadDate(thread.updatedAtMs)} · {thread.segmentCount}
              </span>
            </button>
          ))}
        </nav>

        <footer className="storage-path">{state.appInfo?.dataDir ?? "~/.just-notes"}</footer>
      </aside>

      <section className="thread-panel" aria-label="Transcript">
        <header className="panel-head">
          <div>
            <p className="eyebrow">
              {state.selectedThread ? formatThreadDate(state.selectedThread.summary.createdAtMs) : "Local"}
            </p>
            <h1>{state.selectedThread?.summary.title ?? "No thread selected"}</h1>
          </div>
          <div className="panel-status">
            <span className={state.recorderState === "recording" ? "status-light live" : "status-light"} />
            <span>{formatDuration(state.meters.elapsedMs)}</span>
          </div>
        </header>

        <div className="engine-row">
          <FileText size={15} aria-hidden="true" />
          <span>{state.transcriptionStatus?.message ?? "Checking local transcription"}</span>
        </div>

        <section className="transcript-surface">
          {state.selectedThread && state.selectedThread.segments.length > 0 ? (
            state.selectedThread.segments.map((segment, index) => (
              <article
                key={`${segment.source}-${segment.startMs}-${segment.endMs}-${index}`}
                className="segment"
              >
                <time>{formatDuration(segment.startMs)}</time>
                <strong>{segment.speaker}</strong>
                <p>{segment.text}</p>
              </article>
            ))
          ) : (
            <div className="empty-state">
              <Waves size={24} aria-hidden="true" />
              <span>{state.selectedThread ? "No transcript yet" : "Create or record a thread"}</span>
            </div>
          )}
        </section>

        <footer className="panel-foot">
          <span>{state.liveStatus?.message ?? "Idle"}</span>
          <span>{state.selectedThread?.transcriptMarkdownPath ?? ""}</span>
        </footer>

        {state.error ? <pre className="error">{state.error}</pre> : null}
      </section>
    </main>
  );
}
