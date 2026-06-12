import { eventsApi } from "../api/events";
import { indicatorApi } from "../api/indicator";
import { recordingApi } from "../api/recording";
import { formatDuration } from "../lib/format";
import "../styles/indicator.css";

const pill = document.getElementById("pill") as HTMLDivElement;
const minutes = document.getElementById("minutes") as HTMLSpanElement;
const seconds = document.getElementById("seconds") as HTMLSpanElement;
const stop = document.getElementById("stop") as HTMLButtonElement;

let lastWidth = 0;

// The backend slides the window so exactly the pill's width stays on screen;
// re-sync whenever layout changes (hover expansion, timer growing past 99:59).
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

document.documentElement.addEventListener("mouseenter", () => {
  pill.classList.add("expanded");
  syncWindowWidth();
});

document.documentElement.addEventListener("mouseleave", () => {
  pill.classList.remove("expanded");
  syncWindowWidth();
});

pill.addEventListener("click", () => {
  void indicatorApi.openMainWindow();
});

stop.addEventListener("click", (event) => {
  event.stopPropagation();
  stop.disabled = true;
  void recordingApi.stop();
});

void eventsApi.onMeter((meter) => setElapsed(meter.elapsedMs));
syncWindowWidth();
