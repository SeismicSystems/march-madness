import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { formatEther } from "viem";
import type { PublicGroup } from "../hooks/usePublicGroups";
import type { UseGroupsReturn } from "../hooks/useGroups";
import { NoBracketWarning } from "./NoBracketWarning";

interface PublicGroupsListProps {
  publicGroups: PublicGroup[];
  isLoading: boolean;
  error: string | null;
  groups: UseGroupsReturn;
  walletConnected: boolean;
  isBeforeDeadline: boolean;
  walletBalance: bigint | null;
  hasSubmitted: boolean;
  isSessionHydrating: boolean;
}

function PublicGroupCard({
  group,
  groups,
  walletConnected,
  isBeforeDeadline,
  walletBalance,
  hasSubmitted,
  isSessionHydrating,
}: {
  group: PublicGroup;
  groups: UseGroupsReturn;
  walletConnected: boolean;
  isBeforeDeadline: boolean;
  walletBalance: bigint | null;
  hasSubmitted: boolean;
  isSessionHydrating: boolean;
}) {
  const [joining, setJoining] = useState(false);
  const [joinName, setJoinName] = useState("");
  const [joinError, setJoinError] = useState<string | null>(null);

  const isJoined = useMemo(
    () =>
      groups.joinedGroupIds.includes(Number(group.id)) ||
      groups.joinedGroups.some((jg) => jg.groupId === Number(group.id)),
    [groups.joinedGroupIds, groups.joinedGroups, group.id],
  );

  const entryFeeDisplay = useMemo(() => {
    if (group.entry_fee && BigInt(group.entry_fee) > 0n) {
      return `${formatEther(BigInt(group.entry_fee))} ETH`;
    }
    return "Free";
  }, [group.entry_fee]);

  const [attempted, setAttempted] = useState(false);

  const handleJoin = async () => {
    setAttempted(true);
    if (!joinName.trim()) return;
    setJoinError(null);
    try {
      const groupId = Number(group.id);
      const entryFee = group.entry_fee ? BigInt(group.entry_fee) : 0n;
      if (entryFee > 0n && walletBalance !== null && walletBalance < entryFee) {
        setJoinError(
          `Entry fee is ${formatEther(entryFee)} ETH, but your balance is ${formatEther(walletBalance)} ETH`,
        );
        return;
      }
      await groups.joinGroup(groupId, joinName.trim(), entryFee);
      setAttempted(false);
      setJoining(false);
      setJoinName("");
    } catch (err) {
      setJoinError(err instanceof Error ? err.message : "Failed to join");
    }
  };

  return (
    <div className="rounded-lg bg-bg-tertiary border border-border p-3">
      <div className="flex items-center justify-between">
        <div className="min-w-0 flex-1">
          <span className="font-medium text-text-primary">
            {group.display_name}
          </span>
          <span className="ml-2 text-xs text-text-tertiary">/{group.slug}</span>
        </div>
        <div className="flex items-center gap-3 flex-shrink-0">
          <Link
            to={`/groups/${group.slug}/leaderboard`}
            className="text-xs text-accent hover:text-accent-hover transition-colors"
          >
            Leaderboard
          </Link>
          <span className="text-xs text-text-secondary">
            {group.member_count} member
            {group.member_count !== 1 ? "s" : ""}
          </span>
          <span className="text-xs text-gold min-w-[5rem] text-right">
            {entryFeeDisplay}
          </span>
          {!joining && !isJoined && (
            <button
              onClick={() => {
                if (!walletConnected) return;
                setJoining(true);
                setJoinName("");
                setJoinError(null);
              }}
              disabled={!walletConnected || !isBeforeDeadline}
              title={
                !walletConnected
                  ? "Connect wallet to join"
                  : !isBeforeDeadline
                    ? "Deadline has passed"
                    : "Join this group"
              }
              className={`px-3 py-1 text-xs rounded-lg bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors font-medium min-w-[4rem] ${!walletConnected || !isBeforeDeadline ? "cursor-not-allowed" : "cursor-pointer"}`}
            >
              Join
            </button>
          )}
          {isJoined && (
            <span
              className="px-3 py-1 text-xs rounded-lg bg-green-900/30 text-green-400 border border-green-800/50 font-medium min-w-[4rem] text-center"
              title="You are a member of this group"
            >
              Joined ✓
            </span>
          )}
        </div>
      </div>

      {joining && !isJoined && (
        <div className="mt-3 pt-3 border-t border-border space-y-2">
          {!isSessionHydrating && !hasSubmitted ? (
            <NoBracketWarning />
          ) : (
            <>
              <div className="flex gap-2 max-w-md">
                <input
                  type="text"
                  value={joinName}
                  onChange={(e) => setJoinName(e.target.value)}
                  placeholder="Your display name"
                  className="flex-1 px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleJoin();
                  }}
                />
                <button
                  onClick={handleJoin}
                  disabled={groups.isLoading}
                  className={`px-3 py-1.5 text-sm rounded-lg bg-accent text-white transition-colors font-medium ${groups.isLoading || !joinName.trim() ? "opacity-50 cursor-not-allowed" : "hover:bg-accent-hover cursor-pointer"}`}
                >
                  {groups.isLoading ? "Joining..." : "Confirm"}
                </button>
                <button
                  onClick={() => {
                    setJoining(false);
                    setJoinError(null);
                    setAttempted(false);
                  }}
                  className="px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
                >
                  Cancel
                </button>
              </div>
              {attempted && !joinName.trim() && (
                <p className="text-xs text-red-400">Please enter a display name.</p>
              )}
              {joinError && <p className="text-xs text-red-400">{joinError}</p>}
            </>
          )}
        </div>
      )}
    </div>
  );
}

export function PublicGroupsList({
  publicGroups,
  isLoading,
  error,
  groups,
  walletConnected,
  isBeforeDeadline,
  walletBalance,
  hasSubmitted,
  isSessionHydrating,
}: PublicGroupsListProps) {
  const [search, setSearch] = useState("");

  const filtered = publicGroups.filter((g) => {
    if (!search.trim()) return true;
    const q = search.toLowerCase();
    return (
      g.display_name.toLowerCase().includes(q) ||
      g.slug.toLowerCase().includes(q)
    );
  });

  return (
    <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
      <h2 className="text-lg font-semibold text-text-primary mb-1">
        Public Groups
      </h2>
      <p className="text-sm text-text-muted mb-4">
        Browse and join open groups. No passphrase needed.
      </p>

      <div className="mb-4">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search by name or slug..."
          className="w-full max-w-md px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
        />
      </div>

      {isLoading && (
        <p className="text-sm text-text-muted">Loading groups...</p>
      )}

      {error && (
        <p className="text-sm text-red-400 bg-red-900/20 rounded-lg px-3 py-2">
          {error}
        </p>
      )}

      {!isLoading && !error && filtered.length === 0 && (
        <p className="text-sm text-text-muted">
          {search.trim()
            ? "No groups match your search."
            : "No public groups yet."}
        </p>
      )}

      {!isLoading && filtered.length > 0 && (
        <div className="space-y-2">
          {filtered.map((group) => (
            <PublicGroupCard
              key={group.id}
              group={group}
              groups={groups}
              walletConnected={walletConnected}
              isBeforeDeadline={isBeforeDeadline}
              walletBalance={walletBalance}
              hasSubmitted={hasSubmitted}
              isSessionHydrating={isSessionHydrating}
            />
          ))}
        </div>
      )}
    </div>
  );
}
