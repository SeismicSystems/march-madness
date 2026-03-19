import { useQuery } from "@tanstack/react-query";

import { API_BASE } from "../lib/api";

/**
 * Per-team advance probabilities from the forecaster.
 * Key: team name, Value: [pR64, pR32, pS16, pE8, pF4, pChamp]
 */
export type TeamProbs = Record<string, number[]>;

const POLL_INTERVAL = 30_000; // 30s

export function useTeamProbs() {
  const query = useQuery({
    queryKey: ["team-probs"],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/team-probs`);
      if (res.status === 404) {
        return null;
      }
      if (!res.ok) {
        throw new Error(`Failed to fetch team probs: ${res.status}`);
      }
      return (await res.json()) as TeamProbs;
    },
    refetchInterval: POLL_INTERVAL,
  });

  return {
    teamProbs: query.data ?? null,
    loading: query.isPending,
    error: query.error instanceof Error ? query.error.message : null,
  };
}
