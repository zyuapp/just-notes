import type { StatusTone } from "../lib/statusTone";

type StatusPillProps = {
  tone: StatusTone;
  label: string;
};

export function StatusPill({ tone, label }: StatusPillProps) {
  return (
    <span className={`status-pill status-pill-${tone}`}>
      <i aria-hidden="true" />
      {label}
    </span>
  );
}
