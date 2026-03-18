import { useNow } from "../hooks/useNow";

interface DeadlineCountdownProps {
  /** Unix timestamp in seconds from the contract. */
  deadline: number | null;
  /** Compact inline mode for mobile status rows. */
  compact?: boolean;
}

export function DeadlineCountdown({
  deadline,
  compact = false,
}: DeadlineCountdownProps) {
  const now = useNow();

  if (deadline === null) {
    if (compact) {
      return (
        <span className="text-[10px] font-semibold uppercase tracking-wide text-text-muted">
          Loading...
        </span>
      );
    }

    return (
      <div className="rounded-lg px-4 py-2 text-center border bg-bg-tertiary border-border">
        <div className="text-xs text-text-muted mb-1">Brackets lock in</div>
        <div className="font-mono font-bold text-sm text-text-muted">...</div>
      </div>
    );
  }

  const deadlineMs = deadline * 1000;
  const diff = deadlineMs - now;

  if (diff <= 0) {
    if (compact) {
      return (
        <span className="text-[10px] font-semibold uppercase tracking-wide text-danger">
          Locked
        </span>
      );
    }
    return (
      <div className="bg-danger/10 border border-danger/30 rounded-lg px-4 py-2 text-center">
        <span className="text-danger font-medium text-sm">
          Brackets are locked
        </span>
      </div>
    );
  }

  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff / (1000 * 60 * 60)) % 24);
  const minutes = Math.floor((diff / (1000 * 60)) % 60);
  const seconds = Math.floor((diff / 1000) % 60);

  const isUrgent = diff < 1000 * 60 * 60 * 24; // less than 24 hours

  if (compact) {
    return (
      <div className="flex items-center gap-1.5 whitespace-nowrap">
        <span className="text-[10px] uppercase tracking-wide text-text-muted">
          Locks in
        </span>
        <span
          className={`font-mono text-[10px] font-semibold ${
            isUrgent ? "text-warning" : "text-text-primary"
          }`}
        >
          {days > 0 && `${days}d `}
          {String(hours).padStart(2, "0")}:{String(minutes).padStart(2, "0")}:
          {String(seconds).padStart(2, "0")}
        </span>
      </div>
    );
  }

  return (
    <div
      className={`rounded-lg px-4 py-2 text-center border ${
        isUrgent
          ? "bg-warning/10 border-warning/30"
          : "bg-bg-tertiary border-border"
      }`}
    >
      <div className="text-xs text-text-muted mb-1">Brackets lock in</div>
      <div
        className={`font-mono font-bold text-sm ${
          isUrgent ? "text-warning" : "text-text-primary"
        }`}
      >
        {days > 0 && `${days}d `}
        {String(hours).padStart(2, "0")}:{String(minutes).padStart(2, "0")}:
        {String(seconds).padStart(2, "0")}
      </div>
    </div>
  );
}
