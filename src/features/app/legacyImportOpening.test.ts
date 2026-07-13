import { expect, test } from "bun:test";
import { openLegacyImportWorkflow } from "./legacyImportOpening";

test("opening legacy import dismisses Settings before mounting the import dialog", () => {
  const calls: string[] = [];
  openLegacyImportWorkflow(
    () => calls.push("close settings"),
    () => calls.push("open import"),
  );
  expect(calls).toEqual(["close settings", "open import"]);
});
