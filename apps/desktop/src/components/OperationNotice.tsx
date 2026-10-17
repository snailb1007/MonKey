import React, { useEffect, useRef, useState } from "react";
import { buildGitHubIssueUrl } from "../services/issueReport";
import { handleExternalLinkClick } from "../services/externalUrl";

export type NoticeKind = "success" | "error" | "warning";

interface OperationNoticeProps {
  kind: NoticeKind;
  title: string;
  message: string;
  /** Machine-readable error code badge, e.g. "io_error". */
  code?: string;
  /** Sanitized copyable report text (error details or diagnostics). */
  details?: string;
  /** Issue title used for the prefilled GitHub link. */
  issueTitle?: string;
  /** Short human label of the failed action, e.g. "RGB preview". */
  issueAction?: string;
  retryable?: boolean;
  onRetry?: () => void;
  onDismiss?: () => void;
}

async function copyToClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through to the legacy textarea path below.
  }
  try {
    const doc = document as Document & {
      execCommand?: (command: string) => boolean;
    };
    if (typeof doc.execCommand !== "function") {
      return false;
    }
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.setAttribute("readonly", "");
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    const ok = doc.execCommand("copy");
    document.body.removeChild(ta);
    return ok;
  } catch {
    return false;
  }
}

/**
 * Unified operation feedback banner: success / warning / error states with
 * one-click copy of the sanitized report and a prefilled GitHub issue link.
 *
 * - error/warning render with role="alert", success with role="status".
 * - Client-fixable problems (e.g. invalid_input) should use kind="warning".
 */
export const OperationNotice: React.FC<OperationNoticeProps> = ({
  kind,
  title,
  message,
  code,
  details,
  issueTitle,
  issueAction,
  retryable,
  onRetry,
  onDismiss,
}) => {
  const [copied, setCopied] = useState<boolean>(false);
  const [copyFailed, setCopyFailed] = useState<boolean>(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setCopied(false);
    setCopyFailed(false);
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [details, message]);

  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, []);

  const handleCopy = async () => {
    const ok = await copyToClipboard(details ?? message);
    setCopied(ok);
    setCopyFailed(!ok);
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      setCopied(false);
      setCopyFailed(false);
      timerRef.current = null;
    }, 2500);
  };

  const panelClass =
    kind === "success"
      ? "panel success-panel"
      : kind === "warning"
        ? "panel warn-panel"
        : "panel error-panel";
  const role = kind === "success" ? "status" : "alert";
  const githubUrl =
    kind !== "success" && details
      ? buildGitHubIssueUrl({
          title: issueTitle ?? `Desktop error: ${code ?? "unknown"}`,
          report: details,
          actionLabel: issueAction,
        })
      : null;

  return (
    <div className={panelClass} role={role}>
      <div className="panel-header">
        <span>{title}</span>
        {code && <span className="badge badge-inactive">Code: {code}</span>}
      </div>
      <p className={kind === "success" ? "success-message" : "error-message"}>{message}</p>
      <div className="notice-actions">
        {details && (
          <button type="button" className="pixel-btn small-btn" onClick={handleCopy}>
            {copied ? "Copied to clipboard!" : "Copy details"}
          </button>
        )}
        {githubUrl && (
          <a
            className="notice-link"
            href={githubUrl}
            target="_blank"
            rel="noreferrer"
            onClick={handleExternalLinkClick}
          >
            Report on GitHub
          </a>
        )}
        {retryable && onRetry && (
          <button type="button" className="pixel-btn small-btn" onClick={onRetry}>
            Retry
          </button>
        )}
        {onDismiss && (
          <button type="button" className="pixel-btn small-btn" onClick={onDismiss}>
            Dismiss
          </button>
        )}
      </div>
      {copyFailed && (
        <p className="notice-copy-fallback" role="status">
          Copy blocked by the browser — select the text above manually.
        </p>
      )}
      {githubUrl && (
        <p className="github-hint">
          To file a bug: 1) Copy details, 2) open “Report on GitHub”, 3) paste
          into the issue body.
        </p>
      )}
    </div>
  );
};
