import { useCallback, useEffect, useState } from "react";
import type { TournamentStatus } from "@march-madness/client";

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3000";
const POLL_INTERVAL = 30_000; // 30s

export function useTournamentStatus() {
  const [status, setStatus] = useState<TournamentStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetch_ = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/tournament-status`);
      if (res.ok) {
        const data: TournamentStatus = await res.json();
        setStatus(data);
        setError(null);
      } else if (res.status === 404) {
        setStatus(null);
        setError(null);
      } else {
        setError(`Failed to fetch tournament status: ${res.status}`);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Network error");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetch_();
    const id = setInterval(fetch_, POLL_INTERVAL);
    return () => clearInterval(id);
  }, [fetch_]);

  return { status, loading, error, refetch: fetch_ };
}
