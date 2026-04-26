import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

type ThreadStatus = "idle" | "recording";
type RecorderState = "idle" | "starting" | "recording" | "stopping";

type AppInfo = {
  dataDir: string;
  threadsDir: string;
  fixtureMode: boolean;
};

type ThreadSummary = {
  id: string;
  title: string;
  createdAtMs: number;
  updatedAtMs: number;
  status: ThreadStatus;
  segmentCount: number;
  path: string;
};

type TranscriptSegment = {
  speaker: string;
  source: string;
  startMs: number;
  endMs: number;
  text: string;
};

type ThreadDetail = {
  summary: ThreadSummary;
  segments: TranscriptSegment[];
  transcriptMarkdownPath: string;
};

type TranscriptionStatus = {
  ready: boolean;
  engineExists: boolean;
  modelExists: boolean;
  enginePath: string;
  modelPath: string;
  modelName: string;
  availableModels: WhisperModelStatus[];
  message: string;
};

type WhisperModelStatus = {
  name: string;
  filename: string;
  path: string;
  installed: boolean;
  selected: boolean;
};

type RecordingPayload = {
  thread: ThreadDetail;
  transcription: TranscriptionStatus;
};

type MeterPayload = {
  threadId: string;
  micLevel: number;
  systemLevel: number;
  elapsedMs: number;
};

type LiveTranscriptSegmentPayload = {
  threadId: string;
  committedUntilMs: number;
  segment: TranscriptSegment;
};

type LiveTranscriptStatusPayload = {
  threadId: string;
  active: boolean;
  message: string;
  chunkMs: number;
  overlapMs: number;
};

const meterBars = Array.from({ length: 18 }, (_, index) => index);

function formatDuration(ms: number) {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

function formatThreadDate(ms: number) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(ms));
}

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
    const unlistenMeter = listen<MeterPayload>("meter-update", (event) => {
      setMeters(event.payload);
    });
    const unlistenSegment = listen<LiveTranscriptSegmentPayload>(
      "live-transcript-segment",
      (event) => {
        setSelectedThread((current) => {
          if (!current || current.summary.id !== event.payload.threadId) return current;
          const segments = [...current.segments, event.payload.segment].sort((left, right) => {
            if (left.startMs !== right.startMs) return left.startMs - right.startMs;
            return left.source.localeCompare(right.source);
          });
          return {
            ...current,
            segments,
            summary: {
              ...current.summary,
              segmentCount: segments.length,
              updatedAtMs: Date.now(),
            },
          };
        });
        setThreads((current) =>
          current.map((thread) =>
            thread.id === event.payload.threadId
              ? {
                  ...thread,
                  segmentCount: thread.segmentCount + 1,
                  updatedAtMs: Date.now(),
                }
              : thread,
          ),
        );
      },
    );
    const unlistenStatus = listen<LiveTranscriptStatusPayload>(
      "live-transcript-status",
      (event) => {
        setLiveStatus(event.payload);
      },
    );
    const unlistenError = listen<LiveTranscriptStatusPayload>(
      "live-transcript-error",
      (event) => {
        setLiveStatus(event.payload);
        setError(event.payload.message);
      },
    );

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
        invoke<AppInfo>("get_app_info"),
        invoke<TranscriptionStatus>("get_transcription_status"),
        invoke<ThreadSummary[]>("list_threads"),
      ]);
      setAppInfo(info);
      setTranscriptionStatus(status);
      setThreads(threadList);
      if (threadList.length > 0) {
        await selectThread(threadList[0].id);
      }
    } catch (err) {
      setError(String(err));
    }
  }

  async function refreshThreads(nextSelectedId?: string) {
    const threadList = await invoke<ThreadSummary[]>("list_threads");
    setThreads(threadList);
    const id = nextSelectedId ?? selectedThreadId ?? threadList[0]?.id ?? null;
    if (id) {
      await selectThread(id);
    }
  }

  async function selectThread(threadId: string) {
    setError(null);
    const detail = await invoke<ThreadDetail>("get_thread", { threadId });
    setSelectedThreadId(threadId);
    setSelectedThread(detail);
  }

  async function createThread() {
    setError(null);
    try {
      const detail = await invoke<ThreadDetail>("create_thread");
      setSelectedThreadId(detail.summary.id);
      setSelectedThread(detail);
      await refreshThreads(detail.summary.id);
    } catch (err) {
      setError(String(err));
    }
  }

  async function startRecording() {
    setError(null);
    setRecorderState("starting");
    setMeters({ threadId: "", micLevel: 0, systemLevel: 0, elapsedMs: 0 });

    try {
      const result = await invoke<RecordingPayload>("start_recording", {
        threadId: selectedThreadId,
      });
      setSelectedThreadId(result.thread.summary.id);
      setSelectedThread(result.thread);
      setTranscriptionStatus(result.transcription);
      setRecorderState("recording");
      await refreshThreads(result.thread.summary.id);
    } catch (err) {
      setError(String(err));
      setRecorderState("idle");
    }
  }

  async function startFixtureRecording() {
    setError(null);
    setRecorderState("starting");
    setMeters({ threadId: "", micLevel: 0, systemLevel: 0, elapsedMs: 0 });

    try {
      const result = await invoke<RecordingPayload>("start_fixture_recording", {
        threadId: selectedThreadId,
      });
      setSelectedThreadId(result.thread.summary.id);
      setSelectedThread(result.thread);
      setTranscriptionStatus(result.transcription);
      setRecorderState("recording");
      await refreshThreads(result.thread.summary.id);
    } catch (err) {
      setError(String(err));
      setRecorderState("idle");
    }
  }

  async function stopRecording() {
    setError(null);
    setRecorderState("stopping");

    try {
      const detail = await invoke<ThreadDetail>("stop_recording");
      setSelectedThreadId(detail.summary.id);
      setSelectedThread(detail);
      setMeters((current) => ({ ...current, micLevel: 0, systemLevel: 0 }));
      setRecorderState("idle");
      await refreshThreads(detail.summary.id);
    } catch (err) {
      setError(String(err));
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
