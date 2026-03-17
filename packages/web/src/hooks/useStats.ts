import { useCallback, useEffect, useState } from "react";

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3000";
const POLL_INTERVAL = 30_000; // 30s

export function useStats() {
  const [totalEntries, setTotalEntries] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetch_ = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/stats`);
      if (res.ok) {
        const data: { total_entries: number; scored: number } = await res.json();
        setTotalEntries(data.total_entries);
        setError(null);
      } else {
        setError(`Failed to fetch stats: ${res.status}`);
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

  return { totalEntries, loading, error };
}
