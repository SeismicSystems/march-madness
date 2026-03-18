import { useQuery } from "@tanstack/react-query";

import { API_BASE } from "../lib/api";

class GroupLeaderboardError extends Error {
  constructor(
    message: string,
    readonly notFound: boolean = false,
  ) {
    super(message);
    this.name = "GroupLeaderboardError";
  }
}

interface GroupMembersResult {
  groupName: string;
  members: Set<string>;
}

async function fetchGroupMembers(slug: string): Promise<GroupMembersResult> {
  const [membersRes, groupRes] = await Promise.all([
    fetch(`${API_BASE}/groups/${slug}/members`),
    fetch(`${API_BASE}/groups/${slug}`),
  ]);

  if (membersRes.status === 404 || groupRes.status === 404) {
    throw new GroupLeaderboardError(`Group "/${slug}" not found.`, true);
  }

  if (!membersRes.ok) {
    throw new GroupLeaderboardError(
      `Failed to fetch group members (${membersRes.status})`,
    );
  }

  if (!groupRes.ok) {
    throw new GroupLeaderboardError(
      `Failed to fetch group (${groupRes.status})`,
    );
  }

  const [addresses, group] = (await Promise.all([
    membersRes.json() as Promise<string[]>,
    groupRes.json() as Promise<{ display_name?: string }>,
  ])) as [string[], { display_name?: string }];

  return {
    groupName: group.display_name ?? slug,
    members: new Set(addresses.map((address) => address.toLowerCase())),
  };
}

export function useGroupMembers(slug: string | undefined) {
  const query = useQuery({
    queryKey: ["group-members", slug],
    queryFn: () => fetchGroupMembers(slug!),
    enabled: !!slug,
    retry: false,
  });

  const error =
    query.error instanceof GroupLeaderboardError
      ? query.error.message
      : query.error instanceof Error
        ? query.error.message
        : null;
  const notFound =
    query.error instanceof GroupLeaderboardError && query.error.notFound;

  return {
    members: query.data?.members ?? null,
    groupName: query.data?.groupName ?? null,
    loading: query.isPending,
    error,
    notFound,
    refetch: query.refetch,
  };
}
