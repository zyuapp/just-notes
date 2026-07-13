import type { ReactNode } from "react";
import { useDismissOnEscape } from "./useDismissOnEscape";
import { useModalFocus } from "./useModalFocus";

type ModalFrameProps = {
  className?: string;
  role?: "dialog" | "alertdialog";
  labelledBy: string;
  describedBy: string;
  icon: ReactNode;
  iconTone?: "default" | "success";
  onDismiss?: () => void;
  closeOnBackdrop?: boolean;
  children: ReactNode;
  actions: ReactNode;
};

export function ModalFrame({
  className,
  role = "dialog",
  labelledBy,
  describedBy,
  icon,
  iconTone = "default",
  onDismiss,
  closeOnBackdrop = false,
  children,
  actions,
}: ModalFrameProps) {
  useDismissOnEscape(() => onDismiss?.());
  const { containerRef, handleKeyDown } = useModalFocus<HTMLDivElement>(
    "first",
    document.getElementById("root"),
  );

  return (
    <div
      ref={containerRef}
      className="modal-overlay"
      onKeyDown={handleKeyDown}
      onMouseDown={(event) => {
        if (closeOnBackdrop && event.target === event.currentTarget) onDismiss?.();
      }}
    >
      <section
        className={["modal-frame", className].filter(Boolean).join(" ")}
        role={role}
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-describedby={describedBy}
      >
        <div className={`modal-icon modal-icon-${iconTone}`} aria-hidden="true">
          {icon}
        </div>
        <div className="modal-copy">{children}</div>
        <div className="modal-actions">{actions}</div>
      </section>
    </div>
  );
}
