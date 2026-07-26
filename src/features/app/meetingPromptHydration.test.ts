import { expect, mock, test } from "bun:test";
import type { MeetingPromptPayload } from "../../bindings/MeetingPromptPayload";
import { createMeetingPromptHydration } from "./meetingPromptHydration";

const oldPrompt: MeetingPromptPayload = {
  requestId: "old",
  title: "Old meeting",
  startAtMs: 100,
  endAtMs: 200,
  autoStart: false,
};

const currentPrompt: MeetingPromptPayload = {
  ...oldPrompt,
  requestId: "current",
  title: "Current meeting",
};

test("a live prompt wins over slower initial hydration", async () => {
  const onPrompt = mock();
  const hydration = createMeetingPromptHydration(onPrompt);
  let resolve: (prompt: MeetingPromptPayload) => void = () => undefined;
  const pending = new Promise<MeetingPromptPayload>((done) => {
    resolve = done;
  });

  const loading = hydration.hydrate(() => pending);
  hydration.receive(currentPrompt);
  resolve(oldPrompt);
  await loading;

  expect(onPrompt.mock.calls).toEqual([[currentPrompt]]);
});

test("disposal prevents late hydration updates", async () => {
  const onPrompt = mock();
  const hydration = createMeetingPromptHydration(onPrompt);
  hydration.dispose();

  await hydration.hydrate(async () => oldPrompt);

  expect(onPrompt).not.toHaveBeenCalled();
});
