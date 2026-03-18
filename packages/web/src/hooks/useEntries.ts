import { useQuery } from "@tanstack/react-query";
import type { EntryIndex } from "@march-madness/client";

import { API_BASE } from "../lib/api";

const POLL_INTERVAL = 30_000; // 30s

export function useEntries() {
  const query = useQuery({
    queryKey: ["entries"],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/entries`);
      if (!res.ok) {
        throw new Error(`Failed to fetch entries: ${res.status}`);
      }
      return (await res.json()) as EntryIndex;
    },
    refetchInterval: POLL_INTERVAL,
  });

  return {
    entries: query.data ?? null,
    loading: query.isPending,
    error: query.error instanceof Error ? query.error.message : null,
    refetch: query.refetch,
  };
}
