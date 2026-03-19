import { useQuery } from "@tanstack/react-query";
import type { ForecastIndex } from "@march-madness/client";

import { API_BASE } from "../lib/api";

const POLL_INTERVAL = 30_000;

export function useGroupForecasts(slug: string | undefined) {
  const query = useQuery({
    queryKey: ["group-forecasts", slug],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/forecasts/groups/s/${slug}`);
      if (res.status === 404) return null;
      if (!res.ok)
        throw new Error(`Failed to fetch group forecasts: ${res.status}`);
      return (await res.json()) as ForecastIndex;
    },
    enabled: !!slug,
    refetchInterval: POLL_INTERVAL,
  });

  return {
    forecasts: query.data ?? null,
    loading: query.isPending,
    error: query.error?.message ?? null,
  };
}
