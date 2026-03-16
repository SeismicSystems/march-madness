import { useState } from "react";
import type { JoinedGroup } from "../hooks/useGroups";

interface GroupsSectionProps {
  joinedGroups: JoinedGroup[];
  isLoading: boolean;
  error: string | null;
  isBeforeDeadline: boolean;
  walletConnected: boolean;
  onJoinGroup: (groupId: number, name: string, entryFee: bigint) => Promise<unknown>;
  onLeaveGroup: (groupId: number) => Promise<unknown>;
  onEditEntryName: (groupId: number, name: string) => Promise<unknown>;
  onLookupBySlug: (slug: string) => Promise<[number, unknown] | null>;
  onTrackGroup: (groupId: number) => void;
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
  joinedGroups,
  isLoading,
  error,
  isBeforeDeadline,
  walletConnected,
  onJoinGroup,
  onLeaveGroup,
  onEditEntryName,
  onLookupBySlug,
  onTrackGroup,
}: GroupsSectionProps) {
  const [joinInput, setJoinInput] = useState("");
  const [nameInput, setNameInput] = useState("");
  const [joinError, setJoinError] = useState<string | null>(null);
  const [editingGroup, setEditingGroup] = useState<number | null>(null);
  const [editName, setEditName] = useState("");

  const handleJoin = async () => {
    if (!joinInput.trim() || !nameInput.trim()) return;
    setJoinError(null);

    try {
      // Try parsing as group ID first
      const asNumber = parseInt(joinInput, 10);
      let groupId: number;

      if (!isNaN(asNumber) && String(asNumber) === joinInput.trim()) {
        groupId = asNumber;
      } else {
        // Try slug lookup
        const result = await onLookupBySlug(joinInput.trim());
        if (!result) {
          setJoinError("Group not found");
          return;
        }
        groupId = result[0];
      }

      await onJoinGroup(groupId, nameInput.trim(), 0n);
      setJoinInput("");
      setNameInput("");
    } catch (err) {
      setJoinError(err instanceof Error ? err.message : "Failed to join");
    }
  };

  const handleTrackById = () => {
    const id = parseInt(joinInput, 10);
    if (!isNaN(id)) {
      onTrackGroup(id);
      setJoinInput("");
    }
  };

  const handleEditName = async (groupId: number) => {
    if (!editName.trim()) return;
    try {
      await onEditEntryName(groupId, editName.trim());
      setEditingGroup(null);
      setEditName("");
    } catch {
      // error handled by hook
    }
  };

  return (
    <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
      <h2 className="text-lg font-semibold text-text-primary mb-4">Groups</h2>

      {error && (
        <div className="mb-3 text-sm text-red-400 bg-red-900/20 rounded-lg px-3 py-2">
          {error}
        </div>
      )}

      {/* Joined Groups List */}
      {joinedGroups.length > 0 ? (
        <div className="space-y-3 mb-4">
          {joinedGroups.map(({ groupId, group, members, storedInfo }) => (
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
                </div>
                <span className="text-xs text-text-secondary">
                  {group.entryCount} member{group.entryCount !== 1 ? "s" : ""}
                </span>
              </div>

              {/* Passphrase display for private groups */}
              {storedInfo.passphrase && (
                <div className="flex items-center text-xs text-text-secondary mb-2 bg-bg-primary rounded px-2 py-1">
                  <span className="text-text-tertiary mr-1">Passphrase:</span>
                  <span className="text-text-primary">{storedInfo.passphrase}</span>
                  <CopyButton text={storedInfo.passphrase} />
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
                          disabled={isLoading}
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
                          onClick={() => onLeaveGroup(groupId)}
                          disabled={isLoading}
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
        <p className="text-sm text-text-tertiary mb-4">
          No groups joined yet. Join a group by ID or slug below.
        </p>
      )}

      {/* Join Group Form */}
      {walletConnected && isBeforeDeadline && (
        <div className="space-y-2">
          <div className="flex gap-2">
            <input
              type="text"
              value={joinInput}
              onChange={(e) => setJoinInput(e.target.value)}
              placeholder="Group ID or slug"
              className="flex-1 px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-tertiary"
            />
            <input
              type="text"
              value={nameInput}
              onChange={(e) => setNameInput(e.target.value)}
              placeholder="Your name"
              className="flex-1 px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-tertiary"
            />
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleJoin}
              disabled={isLoading || !joinInput.trim() || !nameInput.trim()}
              className="px-4 py-2 text-sm rounded-lg bg-indigo-600 text-white hover:bg-indigo-500 disabled:opacity-50 transition-colors"
            >
              {isLoading ? "Joining..." : "Join Group"}
            </button>
            <button
              onClick={handleTrackById}
              disabled={!joinInput.trim()}
              className="px-4 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:text-text-primary transition-colors"
              title="Track a group without joining on-chain (e.g. if you already joined)"
            >
              Track Only
            </button>
          </div>
          {joinError && (
            <p className="text-xs text-red-400">{joinError}</p>
          )}
        </div>
      )}
    </div>
  );
}
