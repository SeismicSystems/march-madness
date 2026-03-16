import { useCallback, useEffect, useState } from "react";
import type { ForecastIndex } from "@march-madness/client";

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3000";
const POLL_INTERVAL = 30_000; // 30s

export function useForecasts() {
  const [forecasts, setForecasts] = useState<ForecastIndex | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetch_ = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/forecasts`);
      if (res.ok) {
        const data: ForecastIndex = await res.json();
        setForecasts(data);
        setError(null);
      } else if (res.status === 404) {
        setForecasts(null);
        setError(null);
      } else {
        setError(`Failed to fetch forecasts: ${res.status}`);
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

  return { forecasts, loading, error, refetch: fetch_ };
}
