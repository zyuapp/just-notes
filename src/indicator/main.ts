import { eventsApi } from "../api/events";
import { indicatorApi } from "../api/indicator";
import { recordingApi } from "../api/recording";
import { formatDuration } from "../lib/format";
import "../styles/indicator.css";

const pill = document.getElementById("pill") as HTMLDivElement;
const minutes = document.getElementById("minutes") as HTMLSpanElement;
const seconds = document.getElementById("seconds") as HTMLSpanElement;
const stop = document.getElementById("stop") as HTMLButtonElement;

let recording = false;
let starting = false;
let lastWidth = 0;

// The backend slides the window so exactly the pill's width stays on screen;
// re-sync whenever layout changes (mode switch, hover expansion, timer growth).
function syncWindowWidth() {
  const width = Math.ceil(pill.getBoundingClientRect().width);
  if (width === lastWidth) return;
  lastWidth = width;
  void indicatorApi.setVisibleWidth(width);
}

function setElapsed(elapsedMs: number) {
  const [m, s] = formatDuration(elapsedMs).split(":");
  minutes.textContent = m ?? "00";
  seconds.textContent = s ?? "00";
  syncWindowWidth();
}

function setMode(nextRecording: boolean) {
  recording = nextRecording;
  pill.classList.toggle("recording", recording);
  pill.classList.toggle("idle", !recording);
  pill.title = recording ? "Recording — click to open Just Notes" : "Start a new recording";
  if (!recording) {
    stop.disabled = false;
    setElapsed(0);
  }
  syncWindowWidth();
}

async function startRecording() {
  if (starting) return;
  starting = true;
  try {
    await recordingApi.start(null);
  } catch {
    // The pill has no room for errors; surface the failure in the app.
    void indicatorApi.openMainWindow();
  } finally {
    starting = false;
  }
}

document.documentElement.addEventListener("mouseenter", () => {
  pill.classList.add("expanded");
  syncWindowWidth();
});

document.documentElement.addEventListener("mouseleave", () => {
  pill.classList.remove("expanded");
  syncWindowWidth();
});

pill.addEventListener("click", () => {
  if (recording) {
    void indicatorApi.openMainWindow();
  } else {
    void startRecording();
  }
});

stop.addEventListener("click", (event) => {
  event.stopPropagation();
  stop.disabled = true;
  void recordingApi.stop().catch(() => {
    stop.disabled = false;
  });
});

void eventsApi.onMeter((meter) => setElapsed(meter.elapsedMs));
void eventsApi.onIndicatorState(setMode);
void indicatorApi.getState().then(setMode);
