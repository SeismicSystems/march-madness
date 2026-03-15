import { useCallback, useEffect, useState } from "react";
import type { EntryIndex } from "@march-madness/client";

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3000";
const POLL_INTERVAL = 30_000; // 30s

export function useEntries() {
  const [entries, setEntries] = useState<EntryIndex | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetch_ = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/entries`);
      if (res.ok) {
        const data: EntryIndex = await res.json();
        setEntries(data);
        setError(null);
      } else {
        setError(`Failed to fetch entries: ${res.status}`);
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

  return { entries, loading, error, refetch: fetch_ };
}
