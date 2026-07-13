export type IsolatableElement = {
  inert: boolean;
  getAttribute: (name: string) => string | null;
  setAttribute: (name: string, value: string) => void;
  removeAttribute: (name: string) => void;
};

export function isolateElements(elements: IsolatableElement[]) {
  const previousStates = elements.map((element) => ({
    element,
    inert: element.inert,
    ariaHidden: element.getAttribute("aria-hidden"),
  }));
  for (const element of elements) {
    element.inert = true;
    element.setAttribute("aria-hidden", "true");
  }

  return () => {
    for (const { element, inert, ariaHidden } of previousStates) {
      element.inert = inert;
      if (ariaHidden === null) element.removeAttribute("aria-hidden");
      else element.setAttribute("aria-hidden", ariaHidden);
    }
  };
}

export function focusLoopTarget<T>(
  current: T | null,
  boundary: T,
  first: T | undefined,
  last: T | undefined,
  reverse: boolean,
) {
  if (!first || !last) return null;
  if (reverse && (current === first || current === boundary)) return last;
  if (!reverse && current === last) return first;
  return null;
}
