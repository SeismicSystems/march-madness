import { useCallback, useEffect, useState } from "react";

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3000";

export interface PublicGroup {
  id: string;
  slug: string;
  display_name: string;
  creator: string;
  has_password: boolean;
  member_count: number;
  entry_fee?: string;
}

export function usePublicGroups() {
  const [groups, setGroups] = useState<PublicGroup[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchGroups = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const res = await fetch(`${API_BASE}/groups`);
      if (!res.ok) {
        throw new Error(`Failed to fetch groups (${res.status})`);
      }
      const data: PublicGroup[] = await res.json();
      setGroups(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch groups");
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchGroups();
  }, [fetchGroups]);

  const publicGroups = groups.filter((g) => !g.has_password);

  return { groups, publicGroups, isLoading, error, refetch: fetchGroups };
}
