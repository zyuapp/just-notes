import type { MeetingPromptPayload } from "../../bindings/MeetingPromptPayload";

type Prompt = MeetingPromptPayload | null;

export function createMeetingPromptHydration(onPrompt: (prompt: Prompt) => void) {
  let disposed = false;
  let revision = 0;

  return {
    receive(prompt: Prompt) {
      revision += 1;
      if (!disposed) onPrompt(prompt);
    },
    async hydrate(load: () => Promise<Prompt>) {
      const startingRevision = revision;
      const prompt = await load();
      if (!disposed && startingRevision === revision) onPrompt(prompt);
    },
    dispose() {
      disposed = true;
    },
  };
}
