import { TriangleAlert } from "lucide-react";

export type Notice = {
  message: string;
  actionLabel?: string;
  onAction?: () => void;
};

type NoticeBarProps = {
  notice: Notice | null;
};

export function NoticeBar({ notice }: NoticeBarProps) {
  if (!notice) return null;

  return (
    <div className="notice-bar" role="status">
      <TriangleAlert size={18} aria-hidden="true" />
      <span>{notice.message}</span>
      {notice.actionLabel && notice.onAction && (
        <button type="button" onClick={notice.onAction}>
          {notice.actionLabel}
        </button>
      )}
    </div>
  );
}
