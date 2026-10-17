import React from "react";

interface ActionButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  /** When true, the button shows a pixel spinner + ellipsis and exposes aria-busy. */
  pending?: boolean;
  /**
   * When false, pending only shows the spinner without disabling the button.
   * Use for coalesced actions (e.g. throttled RAM preview) where rapid
   * re-invocation is by design. Defaults to true (double-submit guard).
   */
  blockWhilePending?: boolean;
}

/**
 * Send-action button with a loading state.
 *
 * The base label (children) is always kept in the accessible name — pending
 * only appends an ellipsis — so queries like /Preview in RAM/i keep matching
 * while the spinner is visible.
 */
export const ActionButton: React.FC<ActionButtonProps> = ({
  pending = false,
  blockWhilePending = true,
  children,
  disabled,
  ...rest
}) => (
  <button
    type="button"
    {...rest}
    disabled={disabled || (pending && blockWhilePending)}
    aria-busy={pending || undefined}
  >
    {pending && <span className="btn-spinner" aria-hidden="true" />}
    {children}
    {pending ? "…" : null}
  </button>
);
