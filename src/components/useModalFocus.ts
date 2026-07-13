import { type KeyboardEvent, useEffect, useRef } from "react";
import { focusLoopTarget, isolateElements } from "../lib/modalFocus";

const FOCUSABLE_SELECTOR = [
  "button:not(:disabled)",
  "[href]",
  "input:not(:disabled)",
  "select:not(:disabled)",
  "textarea:not(:disabled)",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

export function useModalFocus<T extends HTMLElement>(
  initialFocus: "first" | "container" = "first",
  isolationRoot: HTMLElement | null = document.getElementById("root"),
) {
  const containerRef = useRef<T>(null);
  const returnFocusRef = useRef(
    document.activeElement instanceof HTMLElement ? document.activeElement : null,
  );

  useEffect(() => {
    const returnFocus = returnFocusRef.current;
    const container = containerRef.current;
    if (!container) return;

    const background = [] as HTMLElement[];
    let branch: HTMLElement = container;
    while (branch !== isolationRoot && branch.parentElement) {
      background.push(
        ...Array.from(branch.parentElement.children).filter(
          (sibling): sibling is HTMLElement =>
            sibling instanceof HTMLElement && sibling !== branch,
        ),
      );
      branch = branch.parentElement;
    }
    const restoreBackground = isolateElements(background);

    if (!container.contains(document.activeElement)) {
      const firstControl =
        container.querySelector<HTMLElement>("[autofocus]") ??
        container.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
      if (initialFocus === "container") container.focus();
      else firstControl?.focus();
    }

    return () => {
      restoreBackground();
      returnFocus?.focus();
    };
  }, [initialFocus, isolationRoot]);

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "Tab") return;
    const controls = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (!first || !last) return;
    const activeElement = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    const target = focusLoopTarget<HTMLElement>(
      activeElement,
      event.currentTarget,
      first,
      last,
      event.shiftKey,
    );
    if (target) {
      event.preventDefault();
      target.focus();
    }
  };

  return { containerRef, handleKeyDown };
}
