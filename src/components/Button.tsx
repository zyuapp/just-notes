import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";

type ButtonVariant = "primary" | "secondary" | "quiet" | "danger" | "icon";
type ButtonSize = "compact" | "regular";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  leadingIcon?: ReactNode;
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = "secondary",
    size = "regular",
    leadingIcon,
    className,
    children,
    type = "button",
    ...props
  },
  ref,
) {
  const classes = [
    "ui-button",
    `ui-button-${variant}`,
    `ui-button-${size}`,
    className,
  ].filter(Boolean).join(" ");

  return (
    <button ref={ref} type={type} className={classes} {...props}>
      {leadingIcon && (
        <span className="ui-button-leading-icon" aria-hidden="true">
          {leadingIcon}
        </span>
      )}
      {children}
    </button>
  );
});
