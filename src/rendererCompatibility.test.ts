import { expect, test } from "bun:test";
import { Window } from "happy-dom";

test("React createRoot renders into a DOM container", async () => {
  const browserWindow = new Window({ url: "http://localhost/" });
  const previousWindow = globalThis.window;
  const previousDocument = globalThis.document;
  const previousNavigator = globalThis.navigator;
  Object.assign(globalThis, {
    window: browserWindow,
    document: browserWindow.document,
    navigator: browserWindow.navigator,
  });

  try {
    const [{ createElement }, { flushSync }, { createRoot }] = await Promise.all([
      import("react"),
      import("react-dom"),
      import("react-dom/client"),
    ]);
    const container = document.createElement("div");
    const root = createRoot(container);

    flushSync(() => root.render(createElement("span", null, "renderer ready")));
    expect(container.textContent).toBe("renderer ready");
    flushSync(() => root.unmount());
    await new Promise<void>((resolve) => setImmediate(resolve));
  } finally {
    browserWindow.close();
    Object.assign(globalThis, {
      window: previousWindow,
      document: previousDocument,
      navigator: previousNavigator,
    });
  }
});
