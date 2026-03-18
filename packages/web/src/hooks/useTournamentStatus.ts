import { useQuery } from "@tanstack/react-query";
import type { TournamentStatus } from "@march-madness/client";

import { API_BASE } from "../lib/api";

const POLL_INTERVAL = 30_000; // 30s

export function useTournamentStatus() {
  const query = useQuery({
    queryKey: ["tournament-status"],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/tournament-status`);
      if (res.status === 404) {
        return null;
      }
      if (!res.ok) {
        throw new Error(`Failed to fetch tournament status: ${res.status}`);
      }
      return (await res.json()) as TournamentStatus;
    },
    refetchInterval: POLL_INTERVAL,
  });

  return {
    status: query.data ?? null,
    loading: query.isPending,
    error: query.error instanceof Error ? query.error.message : null,
    refetch: query.refetch,
  };
}
