import { useQuery } from "@tanstack/react-query";
import type { ForecastIndex } from "@march-madness/client";

import { API_BASE } from "../lib/api";

const POLL_INTERVAL = 30_000; // 30s

/** API returns { address: basisPoints } — transform to ForecastIndex. */
function bpsToForecastIndex(raw: Record<string, number>): ForecastIndex {
  const index: ForecastIndex = {};
  for (const [address, bps] of Object.entries(raw)) {
    index[address] = {
      currentScore: 0,
      maxPossibleScore: 0,
      expectedScore: 0,
      winProbability: bps / 10_000,
    };
  }
  return index;
}

export function useForecasts() {
  const query = useQuery({
    queryKey: ["forecasts"],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/forecasts`);
      if (res.status === 404) {
        return null;
      }
      if (!res.ok) {
        throw new Error(`Failed to fetch forecasts: ${res.status}`);
      }
      const raw = (await res.json()) as Record<string, number>;
      return bpsToForecastIndex(raw);
    },
    refetchInterval: POLL_INTERVAL,
  });

  return {
    forecasts: query.data ?? null,
    loading: query.isPending,
    error: query.error instanceof Error ? query.error.message : null,
    refetch: query.refetch,
  };
}
