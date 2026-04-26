import { Mic, Radio } from "lucide-react";

const meterBars = Array.from({ length: 18 }, (_, index) => index);

type MeterProps = {
  label: string;
  level: number;
  source: "mic" | "system";
};

export function Meter({ label, level, source }: MeterProps) {
  const activeBars = Math.round(Math.min(1, Math.max(0, level)) * meterBars.length);
  const Icon = source === "mic" ? Mic : Radio;

  return (
    <section className="meter" aria-label={`${label} level`}>
      <div className="meter-heading">
        <Icon size={15} aria-hidden="true" />
        <span>{label}</span>
        <strong>{Math.round(level * 100)}%</strong>
      </div>
      <div className="meter-grid" aria-hidden="true">
        {meterBars.map((bar) => (
          <span key={bar} className={bar < activeBars ? "active" : ""} />
        ))}
      </div>
    </section>
  );
}
