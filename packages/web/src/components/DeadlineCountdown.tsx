import { useEffect, useState } from "react";

import { SUBMISSION_DEADLINE } from "../lib/constants";

export function DeadlineCountdown() {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, []);

  const deadlineMs = SUBMISSION_DEADLINE * 1000;
  const diff = deadlineMs - now;

  if (diff <= 0) {
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
