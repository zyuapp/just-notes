import { describe, expect, test } from "bun:test";
import { buildFlightKeyframes, flightControlPoint, rectCenter } from "./archiveFlight";

describe("archiveFlight geometry", () => {
  test("rectCenter returns the middle of the rect", () => {
    expect(rectCenter({ left: 10, top: 20, width: 100, height: 40 })).toEqual({ x: 60, y: 40 });
  });

  test("control point bows above the straight-line midpoint", () => {
    const control = flightControlPoint({ x: 200, y: 100 }, { x: 40, y: 500 });
    expect(control.y).toBeLessThan(300); // midpoint y is 300; the arc lifts it up
  });

  test("flight starts at the origin and lands on the destination", () => {
    const from = { x: 200, y: 100 };
    const to = { x: 40, y: 500 };
    const frames = buildFlightKeyframes(from, to);

    expect(frames).toHaveLength(27);
    expect(frames[0]).toMatchObject({ offset: 0, transform: "translate(0.0px, 0.0px) scale(1.000)" });
    expect(frames.at(-1)).toMatchObject({
      offset: 1,
      transform: "translate(-160.0px, 400.0px) scale(0.120)",
    });
  });

  test("the pill stays opaque through the first half, then fades", () => {
    const frames = buildFlightKeyframes({ x: 0, y: 0 }, { x: 100, y: 100 });
    expect(frames[0].opacity).toBe(1);
    expect(Number(frames.at(-1)!.opacity)).toBeLessThan(1);
  });
});
