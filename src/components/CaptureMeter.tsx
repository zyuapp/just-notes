type CaptureMeterProps = {
  label: string;
  level: number;
};

const captureBars = Array.from({ length: 14 }, (_, index) => index);

export function CaptureMeter({ label, level }: CaptureMeterProps) {
  const activeBars = Math.round(Math.min(1, Math.max(0, level)) * captureBars.length);

  return (
    <div className="capture-meter" aria-label={`${label} level ${Math.round(level * 100)}%`}>
      <b>{label}</b>
      <div className="capture-meter-bars" aria-hidden="true">
        {captureBars.map((bar) => (
          <span key={bar} className={bar < activeBars ? "on" : ""} />
        ))}
      </div>
    </div>
  );
}
