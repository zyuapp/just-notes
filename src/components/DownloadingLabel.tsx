type DownloadingLabelProps = {
  percent: number;
};

export function DownloadingLabel({ percent }: DownloadingLabelProps) {
  return (
    <>
      Downloading <span className="download-percent">{percent}</span>%
    </>
  );
}
