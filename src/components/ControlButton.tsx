import type { ButtonHTMLAttributes, ReactNode } from "react";

interface ControlButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: ReactNode;
  variant?: "primary" | "quiet" | "danger";
}

export function ControlButton({
  icon,
  variant = "quiet",
  children,
  className = "",
  ...props
}: ControlButtonProps) {
  return (
    <button className={`control-button control-button--${variant} ${className}`} {...props}>
      {icon}
      <span>{children}</span>
    </button>
  );
}
