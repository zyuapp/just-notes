import type { AppInfo } from "./bindings/AppInfo";
import type { LiveTranscriptStatusPayload } from "./bindings/LiveTranscriptStatusPayload";
import type { MeterPayload } from "./bindings/MeterPayload";
import type { ThreadDetail } from "./bindings/ThreadDetail";
import type { ThreadSummary } from "./bindings/ThreadSummary";
import type { TranscriptionStatusPayload as TranscriptionStatus } from "./bindings/TranscriptionStatusPayload";
import { api, getApiErrorMessage } from "./api";
import { formatDuration, formatThreadDate } from "./lib/format";
import { applyLiveSegmentToThread, applyLiveSegmentToThreadList } from "./lib/transcript";
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
import { useEffect, useMemo, useState } from "react";

type RecorderState = "idle" | "starting" | "recording" | "stopping";

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
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [threads, setThreads] = useState<ThreadSummary[]>([]);
  const [selectedThreadId, setSelectedThreadId] = useState<string | null>(null);
  const [selectedThread, setSelectedThread] = useState<ThreadDetail | null>(null);
  const [recorderState, setRecorderState] = useState<RecorderState>("idle");
  const [meters, setMeters] = useState<MeterPayload>({
    threadId: "",
    micLevel: 0,
    systemLevel: 0,
    elapsedMs: 0,
  });
  const [liveStatus, setLiveStatus] = useState<LiveTranscriptStatusPayload | null>(null);
  const [transcriptionStatus, setTranscriptionStatus] = useState<TranscriptionStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void bootstrap();
  }, []);

  useEffect(() => {
    const unlistenMeter = api.events.onMeter((payload) => {
      setMeters(payload);
    });
    const unlistenSegment = api.events.onLiveTranscriptSegment((payload) => {
      const updatedAtMs = Date.now();
      setSelectedThread((current) => applyLiveSegmentToThread(current, payload, updatedAtMs));
      setThreads((current) => applyLiveSegmentToThreadList(current, payload, updatedAtMs));
    });
    const unlistenStatus = api.events.onLiveTranscriptStatus((payload) => {
      setLiveStatus(payload);
    });
    const unlistenError = api.events.onLiveTranscriptError((payload) => {
      setLiveStatus(payload);
      setError(payload.message);
    });

    return () => {
      unlistenMeter.then((dispose) => dispose()).catch(() => undefined);
      unlistenSegment.then((dispose) => dispose()).catch(() => undefined);
      unlistenStatus.then((dispose) => dispose()).catch(() => undefined);
      unlistenError.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  const activeThreadId = liveStatus?.active ? liveStatus.threadId : null;
  const canStart = recorderState === "idle";
  const canStop = recorderState === "recording";
  const statusLabel = useMemo(() => {
    if (recorderState === "starting") return "Starting";
    if (recorderState === "recording") return "Recording";
    if (recorderState === "stopping") return "Stopping";
    return "Ready";
  }, [recorderState]);

  async function bootstrap() {
    setError(null);
    try {
      const [info, status, threadList] = await Promise.all([
        api.app.getInfo(),
        api.transcription.getStatus(),
        api.threads.list(),
      ]);
      setAppInfo(info);
      setTranscriptionStatus(status);
      setThreads(threadList);
      if (threadList.length > 0) {
        await selectThread(threadList[0].id);
      }
    } catch (err) {
      setError(getApiErrorMessage(err));
    }
  }

  async function refreshThreads(nextSelectedId?: string) {
    const threadList = await api.threads.list();
    setThreads(threadList);
    const id = nextSelectedId ?? selectedThreadId ?? threadList[0]?.id ?? null;
    if (id) {
      await selectThread(id);
    }
  }

  async function selectThread(threadId: string) {
    setError(null);
    const detail = await api.threads.get(threadId);
    setSelectedThreadId(threadId);
    setSelectedThread(detail);
  }

  async function createThread() {
    setError(null);
    try {
      const detail = await api.threads.create();
      setSelectedThreadId(detail.summary.id);
      setSelectedThread(detail);
      await refreshThreads(detail.summary.id);
    } catch (err) {
      setError(getApiErrorMessage(err));
    }
  }

  async function startRecording() {
    setError(null);
    setRecorderState("starting");
    setMeters({ threadId: "", micLevel: 0, systemLevel: 0, elapsedMs: 0 });

    try {
      const result = await api.recording.start(selectedThreadId);
      setSelectedThreadId(result.thread.summary.id);
      setSelectedThread(result.thread);
      setTranscriptionStatus(result.transcription);
      setRecorderState("recording");
      await refreshThreads(result.thread.summary.id);
    } catch (err) {
      setError(getApiErrorMessage(err));
      setRecorderState("idle");
    }
  }

  async function startFixtureRecording() {
    setError(null);
    setRecorderState("starting");
    setMeters({ threadId: "", micLevel: 0, systemLevel: 0, elapsedMs: 0 });

    try {
      const result = await api.recording.startFixture(selectedThreadId);
      setSelectedThreadId(result.thread.summary.id);
      setSelectedThread(result.thread);
      setTranscriptionStatus(result.transcription);
      setRecorderState("recording");
      await refreshThreads(result.thread.summary.id);
    } catch (err) {
      setError(getApiErrorMessage(err));
      setRecorderState("idle");
    }
  }

  async function stopRecording() {
    setError(null);
    setRecorderState("stopping");

    try {
      const detail = await api.recording.stop();
      setSelectedThreadId(detail.summary.id);
      setSelectedThread(detail);
      setMeters((current) => ({ ...current, micLevel: 0, systemLevel: 0 }));
      setRecorderState("idle");
      await refreshThreads(detail.summary.id);
    } catch (err) {
      setError(getApiErrorMessage(err));
      setRecorderState("recording");
    }
  }

  return (
    <main className="app-shell">
      <aside className="thread-sidebar" aria-label="Threads">
        <header className="sidebar-head">
          <div className="brand">
            <Waves size={18} aria-hidden="true" />
            <span>Just Notes</span>
          </div>
          <button type="button" className="icon-button" onClick={createThread} aria-label="New thread">
            <Plus size={18} aria-hidden="true" />
          </button>
        </header>

        <div className="sidebar-actions">
          <button
            type="button"
            className={recorderState === "recording" ? "record-button recording" : "record-button"}
            onClick={recorderState === "recording" ? stopRecording : startRecording}
            disabled={recorderState === "starting" || recorderState === "stopping"}
            aria-label={recorderState === "recording" ? "Stop recording" : "Start recording"}
          >
            {recorderState === "recording" ? (
              <Square size={16} aria-hidden="true" />
            ) : (
              <Circle size={16} aria-hidden="true" />
            )}
            <span>{recorderState === "recording" ? "Stop" : "Record"}</span>
          </button>
          <div className="capture-state">{statusLabel}</div>
          {appInfo?.fixtureMode && (
            <button
              type="button"
              className="fixture-button"
              onClick={startFixtureRecording}
              disabled={!canStart}
              aria-label="Start QA fixture recording"
            >
              <FlaskConical size={15} aria-hidden="true" />
              <span>QA fixture</span>
            </button>
          )}
        </div>

        <div className="meters">
          <Meter label="Mic" source="mic" level={meters.micLevel} />
          <Meter label="System" source="system" level={meters.systemLevel} />
        </div>

        <div className="thread-search" aria-hidden="true">
          <Search size={14} />
          <span>Threads</span>
        </div>

        <nav className="thread-list" aria-label="Saved threads">
          {threads.map((thread) => (
            <button
              key={thread.id}
              type="button"
              className={thread.id === selectedThreadId ? "thread-item selected" : "thread-item"}
              onClick={() => void selectThread(thread.id)}
            >
              <span className={thread.id === activeThreadId ? "thread-dot active" : "thread-dot"} />
              <span className="thread-title">{thread.title}</span>
              <span className="thread-meta">
                {formatThreadDate(thread.updatedAtMs)} · {thread.segmentCount}
              </span>
            </button>
          ))}
        </nav>

        <footer className="storage-path">{appInfo?.dataDir ?? "~/.just-notes"}</footer>
      </aside>

      <section className="thread-panel" aria-label="Transcript">
        <header className="panel-head">
          <div>
            <p className="eyebrow">{selectedThread ? formatThreadDate(selectedThread.summary.createdAtMs) : "Local"}</p>
            <h1>{selectedThread?.summary.title ?? "No thread selected"}</h1>
          </div>
          <div className="panel-status">
            <span className={recorderState === "recording" ? "status-light live" : "status-light"} />
            <span>{formatDuration(meters.elapsedMs)}</span>
          </div>
        </header>

        <div className="engine-row">
          <FileText size={15} aria-hidden="true" />
          <span>{transcriptionStatus?.message ?? "Checking local transcription"}</span>
        </div>

        <section className="transcript-surface">
          {selectedThread && selectedThread.segments.length > 0 ? (
            selectedThread.segments.map((segment, index) => (
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
              <span>{selectedThread ? "No transcript yet" : "Create or record a thread"}</span>
            </div>
          )}
        </section>

        <footer className="panel-foot">
          <span>{liveStatus?.message ?? "Idle"}</span>
          <span>{selectedThread?.transcriptMarkdownPath ?? ""}</span>
        </footer>

        {error ? <pre className="error">{error}</pre> : null}
      </section>
    </main>
  );
}
