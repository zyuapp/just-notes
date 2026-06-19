import type { ThreadSummary } from "../bindings/ThreadSummary";

export type ThreadGroup = { label: string; threads: ThreadSummary[] };

export function groupThreadsByDay(threads: ThreadSummary[], now = new Date()): ThreadGroup[] {
  // In-place updates (rename, live segments) leave the list out of order between
  // backend refreshes; group labels must stay contiguous or they collide as keys.
  const sorted = [...threads].sort(
    (left, right) => right.updatedAtMs - left.updatedAtMs || right.createdAtMs - left.createdAtMs,
  );
  const groups: ThreadGroup[] = [];
  for (const thread of sorted) {
    const label = dayLabel(new Date(thread.updatedAtMs), now);
    const last = groups[groups.length - 1];
    if (last && last.label === label) {
      last.threads.push(thread);
    } else {
      groups.push({ label, threads: [thread] });
    }
  }
  return groups;
}

// Thread to fall back to once `threadId` is removed: the one after it, or the
// one before if it was last. `undefined` when there is no neighbor or the id is
// not present.
export function neighborThreadId(threads: ThreadSummary[], threadId: string): string | undefined {
  const index = threads.findIndex((thread) => thread.id === threadId);
  if (index === -1) return undefined;
  const neighbor = threads[index + 1] ?? threads[index - 1];
  return neighbor?.id;
}

function dayLabel(date: Date, now: Date): string {
  const dayStart = (value: Date) =>
    new Date(value.getFullYear(), value.getMonth(), value.getDate()).getTime();
  const dayDiff = Math.round((dayStart(now) - dayStart(date)) / 86_400_000);
  if (dayDiff <= 0) return "Today";
  if (dayDiff === 1) return "Yesterday";
  return new Intl.DateTimeFormat(undefined, {
    month: "long",
    day: "numeric",
    ...(date.getFullYear() === now.getFullYear() ? {} : { year: "numeric" }),
  }).format(date);
}
