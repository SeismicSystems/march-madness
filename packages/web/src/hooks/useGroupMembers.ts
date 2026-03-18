import { useCallback, useEffect, useState } from "react";

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:3000";

/**
 * Fetch group member addresses from the server API.
 * Returns null when slug is undefined (global leaderboard mode).
 */
export function useGroupMembers(slug: string | undefined) {
  const [members, setMembers] = useState<Set<string> | null>(null);
  const [groupName, setGroupName] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchMembers = useCallback(async () => {
    if (!slug) {
      setMembers(null);
      setGroupName(null);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      // Fetch members and group metadata in parallel.
      const [membersRes, groupRes] = await Promise.all([
        fetch(`${API_BASE}/groups/${slug}/members`),
        fetch(`${API_BASE}/groups/${slug}`),
      ]);

      if (!membersRes.ok) {
        throw new Error(`Failed to fetch group members (${membersRes.status})`);
      }
      const addrs: string[] = await membersRes.json();
      setMembers(new Set(addrs.map((a) => a.toLowerCase())));

      if (groupRes.ok) {
        const data = await groupRes.json();
        setGroupName(data.display_name ?? slug);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to fetch group");
    } finally {
      setLoading(false);
    }
  }, [slug]);

  useEffect(() => {
    fetchMembers();
  }, [fetchMembers]);

  return { members, groupName, loading, error };
}
