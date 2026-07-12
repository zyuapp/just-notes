import { expect, mock, test } from "bun:test";
import { EscapeDismissStack } from "./escapeDismissStack";

function escapeEvent(type: "keydown" | "keyup", repeat = false) {
  return {
    type,
    key: "Escape",
    repeat,
    preventDefault: mock(() => undefined),
    stopImmediatePropagation: mock(() => undefined),
  };
}

test("Escape dismisses only the topmost registered overlay", () => {
  const stack = new EscapeDismissStack();
  const underlying = mock(() => undefined);
  const topmost = mock(() => undefined);
  stack.register(underlying);
  stack.register(topmost);

  stack.handleKeyUp(escapeEvent("keyup"));

  expect(topmost).toHaveBeenCalledTimes(1);
  expect(underlying).toHaveBeenCalledTimes(0);
});

test("paired keydown and keyup dismiss once", () => {
  const stack = new EscapeDismissStack();
  const dismiss = mock(() => undefined);
  stack.register(dismiss);

  stack.handleKeyDown(escapeEvent("keydown"));
  stack.handleKeyUp(escapeEvent("keyup"));

  expect(dismiss).toHaveBeenCalledTimes(1);
});

test("keyup alone supports the macOS webview Escape path", () => {
  const stack = new EscapeDismissStack();
  const dismiss = mock(() => undefined);
  stack.register(dismiss);

  stack.handleKeyUp(escapeEvent("keyup"));

  expect(dismiss).toHaveBeenCalledTimes(1);
});

test("repeat keydown cannot dismiss the newly exposed overlay", () => {
  const stack = new EscapeDismissStack();
  const underlying = mock(() => undefined);
  stack.register(underlying);
  let unregisterTopmost = () => undefined;
  const topmost = mock(() => unregisterTopmost());
  unregisterTopmost = stack.register(topmost);

  stack.handleKeyDown(escapeEvent("keydown"));
  stack.handleKeyDown(escapeEvent("keydown", true));
  stack.handleKeyUp(escapeEvent("keyup"));

  expect(topmost).toHaveBeenCalledTimes(1);
  expect(underlying).toHaveBeenCalledTimes(0);
});

test("unregistered overlays are not dismissed", () => {
  const stack = new EscapeDismissStack();
  const dismiss = mock(() => undefined);
  const unregister = stack.register(dismiss);
  unregister();

  stack.handleKeyUp(escapeEvent("keyup"));

  expect(dismiss).toHaveBeenCalledTimes(0);
  expect(stack.size).toBe(0);
});
