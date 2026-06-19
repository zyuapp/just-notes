export type Point = { x: number; y: number };

type RectLike = { left: number; top: number; width: number; height: number };

const SAMPLE_COUNT = 26;
const END_SCALE = 0.12;
const FADE_START = 0.55;
const END_OPACITY = 0.25;

export const FLIGHT_DURATION_MS = 520;
export const FLIGHT_EASING = "cubic-bezier(0.42, 0, 0.78, 1)";
export const CATCH_DURATION_MS = 460;

const ARC_X_BIAS = 0.12; // nudges the apex toward the destination
const ARC_LIFT_RATIO = 0.34; // apex height as a fraction of flight distance
const MIN_ARC_LIFT = 60; // px, so short flights still arc visibly

export function rectCenter(rect: RectLike): Point {
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

// Lifts the path's apex above the straight line so the pill arcs like a toss.
export function flightControlPoint(from: Point, to: Point): Point {
  const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };
  const distance = Math.hypot(to.x - from.x, to.y - from.y);
  return {
    x: mid.x + ARC_X_BIAS * (to.x - from.x),
    y: mid.y - Math.max(MIN_ARC_LIFT, distance * ARC_LIFT_RATIO),
  };
}

function quadraticPoint(from: Point, control: Point, to: Point, t: number): Point {
  const mt = 1 - t;
  return {
    x: mt * mt * from.x + 2 * mt * t * control.x + t * t * to.x,
    y: mt * mt * from.y + 2 * mt * t * control.y + t * t * to.y,
  };
}

// Densely samples the bezier so a translate-per-keyframe animation reads as a smooth curve.
export function buildFlightKeyframes(from: Point, to: Point): Keyframe[] {
  const control = flightControlPoint(from, to);
  const frames: Keyframe[] = [];
  for (let i = 0; i <= SAMPLE_COUNT; i++) {
    const t = i / SAMPLE_COUNT;
    const point = quadraticPoint(from, control, to, t);
    const scale = 1 + (END_SCALE - 1) * (t * t);
    const opacity =
      t < FADE_START ? 1 : 1 + (END_OPACITY - 1) * ((t - FADE_START) / (1 - FADE_START));
    const dx = (point.x - from.x).toFixed(1);
    const dy = (point.y - from.y).toFixed(1);
    frames.push({
      offset: t,
      transform: `translate(${dx}px, ${dy}px) scale(${scale.toFixed(3)})`,
      opacity,
    });
  }
  return frames;
}

export const CATCH_BOUNCE: Keyframe[] = [
  { offset: 0, transform: "translateY(0)" },
  { offset: 0.3, transform: "translateY(-4px)" },
  { offset: 0.55, transform: "translateY(0)" },
  { offset: 0.75, transform: "translateY(-2px)" },
  { offset: 1, transform: "translateY(0)" },
];

export const CATCH_RING: Keyframe[] = [
  { offset: 0, boxShadow: "0 0 0 0 rgba(78, 140, 234, 0)" },
  { offset: 0.25, boxShadow: "0 0 0 6px rgba(78, 140, 234, 0.18)" },
  { offset: 1, boxShadow: "0 0 0 0 rgba(78, 140, 234, 0)" },
];
