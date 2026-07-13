import { describe, expect, test } from "bun:test";
import { focusLoopTarget, isolateElements, type IsolatableElement } from "./modalFocus";

class FakeElement implements IsolatableElement {
  inert = false;
  private attributes = new Map<string, string>();

  getAttribute(name: string) {
    return this.attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string) {
    this.attributes.set(name, value);
  }

  removeAttribute(name: string) {
    this.attributes.delete(name);
  }
}

describe("focusLoopTarget", () => {
  const boundary = { id: "boundary" };
  const first = { id: "first" };
  const last = { id: "last" };

  test("wraps forward from the last control", () => {
    expect(focusLoopTarget(last, boundary, first, last, false)).toBe(first);
  });

  test("wraps backward from the first control or container boundary", () => {
    expect(focusLoopTarget(first, boundary, first, last, true)).toBe(last);
    expect(focusLoopTarget(boundary, boundary, first, last, true)).toBe(last);
  });

  test("leaves interior focus unchanged", () => {
    expect(focusLoopTarget({ id: "middle" }, boundary, first, last, false)).toBeNull();
  });
});

test("isolateElements restores existing inert and aria-hidden states", () => {
  const plain = new FakeElement();
  const alreadyHidden = new FakeElement();
  alreadyHidden.inert = true;
  alreadyHidden.setAttribute("aria-hidden", "menu");

  const restore = isolateElements([plain, alreadyHidden]);
  expect(plain.inert).toBeTrue();
  expect(plain.getAttribute("aria-hidden")).toBe("true");
  restore();

  expect(plain.inert).toBeFalse();
  expect(plain.getAttribute("aria-hidden")).toBeNull();
  expect(alreadyHidden.inert).toBeTrue();
  expect(alreadyHidden.getAttribute("aria-hidden")).toBe("menu");
});
