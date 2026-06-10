import { Mic, Radio } from "lucide-react";

type CaptureMeterProps = {
  icon: "mic" | "system";
  label: string;
  level: number;
};

const captureBars = Array.from({ length: 24 }, (_, index) => index);

export function CaptureMeter({ icon, label, level }: CaptureMeterProps) {
  const Icon = icon === "mic" ? Mic : Radio;
  const activeBars = Math.round(Math.min(1, Math.max(0, level)) * captureBars.length);

  return (
    <section className="capture-meter" aria-label={`${label} level`}>
      <div className="capture-meter-head">
        <Icon size={20} aria-hidden="true" />
        <span>{label}</span>
        <strong>{Math.round(level * 100)}%</strong>
      </div>
      <div className="capture-meter-bars" aria-hidden="true">
        {captureBars.map((bar) => (
          <span key={bar} className={bar < activeBars ? "active" : ""} />
        ))}
      </div>
    </section>
  );
}
