import { useRef } from "react";
import {
  buildFlightKeyframes,
  CATCH_BOUNCE,
  CATCH_DURATION_MS,
  CATCH_RING,
  FLIGHT_DURATION_MS,
  FLIGHT_EASING,
  rectCenter,
} from "../lib/archiveFlight";

// Distance from the row's left edge to where its title begins; the pill lifts off from there.
const PILL_LEFT_INSET = 14;

function playCatch(icon: HTMLElement) {
  icon.animate(CATCH_BOUNCE, { duration: CATCH_DURATION_MS, easing: "ease" });
  icon.animate(CATCH_RING, { duration: CATCH_DURATION_MS, easing: "ease-out" });
}

function runFlight(source: HTMLElement, icon: HTMLElement) {
  const ghost = document.createElement("div");
  ghost.className = "archive-fly";
  const dot = document.createElement("span");
  dot.className = "archive-fly-dot";
  const label = document.createElement("span");
  label.className = "archive-fly-title";
  label.textContent = source.querySelector(".thread-title")?.textContent || "Untitled thread";
  ghost.append(dot, label);
  ghost.style.visibility = "hidden";
  document.body.appendChild(ghost);

  const sourceRect = source.getBoundingClientRect();
  const pill = ghost.getBoundingClientRect();
  const from = {
    x: sourceRect.left + PILL_LEFT_INSET + pill.width / 2,
    y: sourceRect.top + sourceRect.height / 2,
  };
  const to = rectCenter(icon.getBoundingClientRect());
  ghost.style.left = `${from.x - pill.width / 2}px`;
  ghost.style.top = `${from.y - pill.height / 2}px`;
  ghost.style.visibility = "visible";

  const anim = ghost.animate(buildFlightKeyframes(from, to), {
    duration: FLIGHT_DURATION_MS,
    easing: FLIGHT_EASING,
    fill: "forwards",
  });
  const remove = () => ghost.remove();
  anim.finished.then(() => {
    remove();
    playCatch(icon);
  }, remove);
}

export function useArchiveFlight() {
  const iconRef = useRef<HTMLButtonElement>(null);
  const scopeRef = useRef<HTMLElement>(null);

  const flyToArchive = (threadId: string) => {
    const icon = iconRef.current;
    if (!icon) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const source = scopeRef.current?.querySelector<HTMLElement>(
      `[data-thread-id="${CSS.escape(threadId)}"]`,
    );
    if (!source) {
      playCatch(icon);
      return;
    }
    runFlight(source, icon);
  };

  return { iconRef, scopeRef, flyToArchive };
}
