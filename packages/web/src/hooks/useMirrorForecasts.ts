import { useQuery } from "@tanstack/react-query";
import type { ForecastIndex } from "@march-madness/client";

import { API_BASE } from "../lib/api";

const POLL_INTERVAL = 30_000;

export function useMirrorForecasts(id: string | undefined) {
  const query = useQuery({
    queryKey: ["mirror-forecasts", id],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/forecasts/mirrors/id/${id}`);
      if (res.status === 404) return null;
      if (!res.ok)
        throw new Error(`Failed to fetch mirror forecasts: ${res.status}`);
      return (await res.json()) as ForecastIndex;
    },
    enabled: !!id,
    refetchInterval: POLL_INTERVAL,
  });

  return {
    forecasts: query.data ?? null,
    loading: query.isPending,
    error: query.error?.message ?? null,
  };
}
