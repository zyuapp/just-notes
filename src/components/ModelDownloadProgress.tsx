import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import { downloadPercent } from "../lib/transcriptionModel";
import { formatBytes } from "../lib/format";

type ModelDownloadProgressProps = {
  model: TranscriptionModelStatus;
};

export function ModelDownloadProgress({ model }: ModelDownloadProgressProps) {
  const percent = downloadPercent(model);
  return (
    <div className="model-progress">
      <div
        className="model-progress-track"
        role="progressbar"
        aria-label="Model download"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <i style={{ width: `${percent}%` }} />
      </div>
      <div className="model-progress-meta">
        <span>
          {formatBytes(model.progressBytes)} of {formatBytes(model.totalBytes)}
        </span>
        <span className="download-percent">{percent}%</span>
      </div>
    </div>
  );
}
