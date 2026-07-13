import type { ReactNode } from "react";
import { useDismissOnEscape } from "./useDismissOnEscape";
import { useModalFocus } from "./useModalFocus";

type FullscreenViewProps = {
  ariaLabel: string;
  navigation: ReactNode;
  title: string;
  description: string;
  panelClassName?: string;
  onClose: () => void;
  children: ReactNode;
};

export function FullscreenView({
  ariaLabel,
  navigation,
  title,
  description,
  panelClassName,
  onClose,
  children,
}: FullscreenViewProps) {
  useDismissOnEscape(onClose);
  const { containerRef, handleKeyDown } = useModalFocus<HTMLDivElement>(
    "container",
    document.getElementById("root"),
  );

  return (
    <div
      ref={containerRef}
      className="fullscreen-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={ariaLabel}
      tabIndex={-1}
      onKeyDown={handleKeyDown}
    >
      {navigation}
      <div className="fullscreen-content">
        <div className={["fullscreen-panel", panelClassName].filter(Boolean).join(" ")}>
          <header className="fullscreen-titlebar">
            <h2>{title}</h2>
            <p>{description}</p>
          </header>
          {children}
        </div>
      </div>
    </div>
  );
}
