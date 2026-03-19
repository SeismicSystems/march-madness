import { useQuery } from "@tanstack/react-query";

import { API_BASE } from "../lib/api";

interface MirrorInfo {
  id: string;
  slug: string;
  display_name: string;
  admin: string;
  entry_count: number;
}

export interface MirrorEntry {
  slug: string;
  bracket: string;
}

const POLL_INTERVAL = 30_000;

export function useMirror(id: string | undefined) {
  const infoQuery = useQuery({
    queryKey: ["mirror", id],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/mirrors/id/${id}`);
      if (res.status === 404) return null;
      if (!res.ok) throw new Error(`Failed to fetch mirror: ${res.status}`);
      return (await res.json()) as MirrorInfo;
    },
    enabled: !!id,
    retry: false,
  });

  const entriesQuery = useQuery({
    queryKey: ["mirror-entries", id],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/mirrors/id/${id}/entries`);
      if (res.status === 404) return null;
      if (!res.ok)
        throw new Error(`Failed to fetch mirror entries: ${res.status}`);
      return (await res.json()) as MirrorEntry[];
    },
    enabled: !!id,
    refetchInterval: POLL_INTERVAL,
  });

  return {
    mirror: infoQuery.data ?? null,
    entries: entriesQuery.data ?? null,
    loading: infoQuery.isLoading || entriesQuery.isLoading,
    notFound: infoQuery.data === null && !infoQuery.isLoading,
    error: infoQuery.error?.message ?? entriesQuery.error?.message ?? null,
  };
}
