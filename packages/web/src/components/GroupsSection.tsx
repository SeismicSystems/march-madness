import { useState } from "react";
import type { UseGroupsReturn } from "../hooks/useGroups";
import { formatEther } from "viem";

interface GroupsSectionProps {
  groups: UseGroupsReturn;
  isBeforeDeadline: boolean;
  walletConnected: boolean;
  walletBalance: bigint | null;
  initialSlug?: string;
  initialPassphrase?: string;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <button
      onClick={handleCopy}
      className="ml-1 px-1 text-xs text-text-tertiary hover:text-text-primary transition-colors"
      title="Copy to clipboard"
    >
      {copied ? "Copied!" : "Copy"}
    </button>
  );
}

export function GroupsSection({
  groups,
  isBeforeDeadline,
  walletConnected,
  walletBalance,
  initialSlug = "",
  initialPassphrase = "",
}: GroupsSectionProps) {
  const [slugInput, setSlugInput] = useState(initialSlug);
  const [nameInput, setNameInput] = useState("");
  const [passphraseInput, setPassphraseInput] = useState(initialPassphrase);
  const [joinError, setJoinError] = useState<string | null>(null);
  const [editingGroup, setEditingGroup] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [resolvedGroupNeedsPassword, setResolvedGroupNeedsPassword] = useState<boolean | null>(
    initialPassphrase ? true : null,
  );
  // Track by ID form
  const [trackSlugInput, setTrackSlugInput] = useState("");
  const [trackError, setTrackError] = useState("");

  /** Resolve a slug to [groupId, GroupData]. Returns null if not found. */
  const resolveGroup = async (input: string) => {
    const trimmed = input.trim();
    if (!trimmed) return null;
    return await groups.lookupGroupBySlug(trimmed);
  };

  const handleJoin = async () => {
    if (!slugInput.trim() || !nameInput.trim()) return;
    setJoinError(null);

    try {
      // 1. Resolve group
      const result = await resolveGroup(slugInput);
      if (!result) {
        setJoinError("Group not found");
        return;
      }

      const [groupId, groupData] = result;

      // 2. Check if password is needed
      if (groupData.hasPassword && !passphraseInput.trim()) {
        setResolvedGroupNeedsPassword(true);
        setJoinError("This is a private group — enter the passphrase to join");
        return;
      }

      // 3. Check balance vs entry fee
      const entryFee = groupData.entryFee;
      if (entryFee > 0n) {
        if (walletBalance === null) {
          setJoinError("Unable to check wallet balance — try again");
          return;
        }
        if (walletBalance < entryFee) {
          setJoinError(
            `Entry fee is ${formatEther(entryFee)} ETH, but your balance is ${formatEther(walletBalance)} ETH`,
          );
          return;
        }
      }

      // 4. Send the transaction with the correct entry fee
      if (groupData.hasPassword || passphraseInput.trim()) {
        await groups.joinGroupWithPassword(groupId, passphraseInput.trim(), nameInput.trim(), entryFee);
      } else {
        await groups.joinGroup(groupId, nameInput.trim(), entryFee);
      }

      setSlugInput("");
      setNameInput("");
      setPassphraseInput("");
      setResolvedGroupNeedsPassword(null);
    } catch (err) {
      setJoinError(err instanceof Error ? err.message : "Failed to join");
    }
  };

  const handleTrackBySlug = async () => {
    const slug = trackSlugInput.trim();
    if (!slug) return;
    setTrackError("");
    const result = await groups.lookupGroupBySlug(slug);
    if (result) {
      const [groupId] = result;
      groups.trackGroup(groupId);
      setTrackSlugInput("");
    } else {
      setTrackError(`No group found with slug "${slug}"`);
    }
  };

  const handleEditName = async (groupId: number) => {
    if (!editName.trim()) return;
    try {
      await groups.editEntryName(groupId, editName.trim());
      setEditingGroup(null);
      setEditName("");
    } catch {
      // error handled by hook
    }
  };

  return (
    <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
      <h2 className="text-lg font-semibold text-text-primary mb-4">Groups</h2>

      {groups.error && (
        <div className="mb-3 text-sm text-red-400 bg-red-900/20 rounded-lg px-3 py-2">
          {groups.error}
        </div>
      )}

      {/* Joined Groups List */}
      {groups.joinedGroups.length > 0 ? (
        <div className="space-y-3 mb-4">
          {groups.joinedGroups.map(({ groupId, group, members, storedInfo }) => (
            <div
              key={groupId}
              className="rounded-lg bg-bg-tertiary border border-border p-3"
            >
              <div className="flex items-center justify-between mb-2">
                <div>
                  <span className="font-medium text-text-primary">
                    {group.displayName}
                  </span>
                  <span className="ml-2 text-xs text-text-tertiary">
                    /{group.slug}
                  </span>
                  {storedInfo.admin && (
                    <span className="ml-2 text-xs text-indigo-400">Admin</span>
                  )}
                  {group.entryFee > 0n && (
                    <span className="ml-2 text-xs text-amber-400">
                      {formatEther(group.entryFee)} ETH
                    </span>
                  )}
                </div>
                <span className="text-xs text-text-secondary">
                  {group.entryCount} member{group.entryCount !== 1 ? "s" : ""}
                </span>
              </div>

              {/* Passphrase + invite link for private groups */}
              {storedInfo.passphrase && (
                <div className="space-y-1 mb-2">
                  <div className="flex items-center text-xs text-text-secondary bg-bg-primary rounded px-2 py-1">
                    <span className="text-text-tertiary mr-1">Passphrase:</span>
                    <span className="text-text-primary">{storedInfo.passphrase}</span>
                    <CopyButton text={storedInfo.passphrase} />
                  </div>
                  <div className="flex items-center text-xs text-text-secondary bg-bg-primary rounded px-2 py-1">
                    <span className="text-text-tertiary mr-1">Invite link:</span>
                    <span className="text-text-primary truncate">
                      {`${window.location.origin}/groups?slug=${encodeURIComponent(group.slug)}&password=${encodeURIComponent(storedInfo.passphrase)}`}
                    </span>
                    <CopyButton
                      text={`${window.location.origin}/groups?slug=${encodeURIComponent(group.slug)}&password=${encodeURIComponent(storedInfo.passphrase)}`}
                    />
                  </div>
                </div>
              )}

              {/* Member list (compact) */}
              <div className="text-xs text-text-secondary space-y-0.5 mb-2">
                {members.slice(0, 5).map((m, i) => (
                  <div key={i} className="flex justify-between">
                    <span>{m.name || m.addr.slice(0, 10) + "..."}</span>
                    {m.isScored && (
                      <span className="text-text-tertiary">Score: {m.score}</span>
                    )}
                  </div>
                ))}
                {members.length > 5 && (
                  <div className="text-text-tertiary">
                    +{members.length - 5} more
                  </div>
                )}
              </div>

              {/* Actions */}
              <div className="flex gap-2 mt-2">
                {isBeforeDeadline && (
                  <>
                    {editingGroup === groupId ? (
                      <div className="flex gap-1 flex-1">
                        <input
                          type="text"
                          value={editName}
                          onChange={(e) => setEditName(e.target.value)}
                          placeholder="New display name"
                          className="flex-1 px-2 py-1 text-xs rounded bg-bg-primary border border-border text-text-primary"
                        />
                        <button
                          onClick={() => handleEditName(groupId)}
                          disabled={groups.isLoading}
                          className="px-2 py-1 text-xs rounded bg-indigo-600 text-white hover:bg-indigo-500 disabled:opacity-50"
                        >
                          Save
                        </button>
                        <button
                          onClick={() => setEditingGroup(null)}
                          className="px-2 py-1 text-xs rounded bg-bg-primary border border-border text-text-secondary hover:text-text-primary"
                        >
                          Cancel
                        </button>
                      </div>
                    ) : (
                      <>
                        <button
                          onClick={() => {
                            setEditingGroup(groupId);
                            setEditName("");
                          }}
                          className="px-2 py-1 text-xs rounded bg-bg-primary border border-border text-text-secondary hover:text-text-primary"
                        >
                          Edit Name
                        </button>
                        <button
                          onClick={() => groups.leaveGroup(groupId)}
                          disabled={groups.isLoading}
                          className="px-2 py-1 text-xs rounded bg-red-900/30 border border-red-800 text-red-400 hover:bg-red-900/50 disabled:opacity-50"
                        >
                          Leave
                        </button>
                      </>
                    )}
                  </>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-text-muted mb-4">
          No groups joined yet. Join one below, or create a new group from the{" "}
          <a href="/groups" className="text-accent hover:underline">Groups</a> page.
        </p>
      )}

      {/* Join Group Form */}
      {walletConnected && isBeforeDeadline && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium text-text-secondary">Join a Group</h3>
          <div className="space-y-2">
            <input
              type="text"
              value={slugInput}
              onChange={(e) => setSlugInput(e.target.value)}
              placeholder="Group slug"
              className="w-full max-w-md px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
            />
            <input
              type="text"
              value={nameInput}
              onChange={(e) => setNameInput(e.target.value)}
              placeholder="Your display name"
              className="w-full max-w-md px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
            />
            <input
              type="text"
              value={passphraseInput}
              onChange={(e) => setPassphraseInput(e.target.value)}
              placeholder={resolvedGroupNeedsPassword ? "Passphrase (required)" : "Passphrase (leave blank for public groups)"}
              className={`w-full max-w-md px-3 py-1.5 text-sm rounded-lg bg-bg-primary border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors ${
                resolvedGroupNeedsPassword ? "border-amber-500/50" : "border-border"
              }`}
            />
            <div>
              <button
                onClick={handleJoin}
                disabled={groups.isLoading || !slugInput.trim() || !nameInput.trim()}
                className="px-4 py-1.5 text-sm rounded-lg bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors font-medium"
              >
                {groups.isLoading ? "Joining..." : "Join"}
              </button>
            </div>
          </div>
          {joinError && (
            <p className="text-xs text-red-400">{joinError}</p>
          )}

          {/* Track by slug (separate, small) */}
          <div className="pt-2 border-t border-border">
            <h4 className="text-xs text-text-tertiary mb-1">Already a member? Track by slug</h4>
            <div className="flex gap-2">
              <input
                type="text"
                value={trackSlugInput}
                onChange={(e) => { setTrackSlugInput(e.target.value); setTrackError(""); }}
                placeholder="group-slug"
                className="flex-1 px-3 py-1 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
              />
              <button
                onClick={handleTrackBySlug}
                disabled={!trackSlugInput.trim()}
                className="px-3 py-1 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:text-text-primary transition-colors"
              >
                Track
              </button>
            </div>
            {trackError && <p className="text-xs text-red-400 mt-1">{trackError}</p>}
          </div>
        </div>
      )}
    </div>
  );
}
