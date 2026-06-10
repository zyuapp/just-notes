import type { ThreadSummary } from "../bindings/ThreadSummary";

export type ThreadGroup = { label: string; threads: ThreadSummary[] };

export function groupThreadsByDay(threads: ThreadSummary[], now = new Date()): ThreadGroup[] {
  const groups: ThreadGroup[] = [];
  for (const thread of threads) {
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
