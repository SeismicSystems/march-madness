import { useState } from "react";
import { Link } from "react-router-dom";
import type { JoinedGroup, UseGroupsReturn } from "../hooks/useGroups";
import { formatEther } from "viem";

interface GroupsSectionProps {
  groups: UseGroupsReturn;
  isBeforeDeadline: boolean;
  walletConnected: boolean;
  walletBalance: bigint | null;
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

function JoinedGroupCard({
  groupId,
  group,
  members,
  storedInfo,
  isBeforeDeadline,
  isLoading,
  onEditName,
  onLeave,
}: JoinedGroup & {
  isBeforeDeadline: boolean;
  isLoading: boolean;
  onEditName: (groupId: number, name: string) => void;
  onLeave: (groupId: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");

  const inviteLink = storedInfo.passphrase
    ? `${window.location.origin}/groups?slug=${encodeURIComponent(group.slug)}&password=${encodeURIComponent(storedInfo.passphrase)}`
    : null;

  return (
    <div className="rounded-lg bg-bg-tertiary border border-border p-3">
      <div className="flex items-center justify-between mb-2">
        <div>
          <span className="font-medium text-text-primary">
            {group.displayName}
          </span>
          <span className="ml-2 text-xs text-text-tertiary">/{group.slug}</span>
          {storedInfo.admin && (
            <span className="ml-2 text-xs text-accent">Admin</span>
          )}
          {group.entryFee > 0n && (
            <span className="ml-2 text-xs text-gold">
              {formatEther(group.entryFee)} ETH
            </span>
          )}
        </div>
        <span className="text-xs text-text-secondary">
          {group.entryCount} member{group.entryCount !== 1 ? "s" : ""}
        </span>
      </div>

      {storedInfo.passphrase && inviteLink && (
        <div className="space-y-1 mb-2">
          <div className="flex items-center text-xs text-text-secondary bg-bg-primary rounded px-2 py-1">
            <span className="text-text-tertiary mr-1">Passphrase:</span>
            <span className="text-text-primary">{storedInfo.passphrase}</span>
            <CopyButton text={storedInfo.passphrase} />
          </div>
          <div className="flex items-center text-xs text-text-secondary bg-bg-primary rounded px-2 py-1">
            <span className="text-text-tertiary mr-1">Invite link:</span>
            <span className="text-text-primary truncate">{inviteLink}</span>
            <CopyButton text={inviteLink} />
          </div>
        </div>
      )}

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
          <div className="text-text-tertiary">+{members.length - 5} more</div>
        )}
      </div>

      <Link
        to={`/groups/${group.slug}/leaderboard`}
        className="inline-block mb-2 text-xs text-accent hover:text-accent-hover transition-colors"
      >
        View Leaderboard
      </Link>

      {isBeforeDeadline && (
        <div className="flex gap-2 mt-2">
          {editing ? (
            <div className="flex gap-1 flex-1">
              <input
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                placeholder="New display name"
                className="flex-1 px-2 py-1 text-xs rounded bg-bg-primary border border-border text-text-primary"
              />
              <button
                onClick={() => {
                  if (editName.trim()) onEditName(groupId, editName.trim());
                  setEditing(false);
                  setEditName("");
                }}
                disabled={isLoading}
                className="px-2 py-1 text-xs rounded bg-accent text-white hover:bg-accent-hover disabled:opacity-50"
              >
                Save
              </button>
              <button
                onClick={() => setEditing(false)}
                className="px-2 py-1 text-xs rounded bg-bg-primary border border-border text-text-secondary hover:text-text-primary"
              >
                Cancel
              </button>
            </div>
          ) : (
            <>
              <button
                onClick={() => {
                  setEditing(true);
                  setEditName("");
                }}
                className="px-2 py-1 text-xs rounded bg-bg-primary border border-border text-text-secondary hover:text-text-primary"
              >
                Edit Name
              </button>
              <button
                onClick={() => onLeave(groupId)}
                disabled={isLoading}
                className="px-2 py-1 text-xs rounded bg-red-900/30 border border-red-800 text-red-400 hover:bg-red-900/50 disabled:opacity-50"
              >
                Leave
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}

export function GroupsSection({
  groups,
  isBeforeDeadline,
}: GroupsSectionProps) {
  const [trackSlugInput, setTrackSlugInput] = useState("");
  const [trackError, setTrackError] = useState("");

  const handleTrackBySlug = async () => {
    const slug = trackSlugInput.trim();
    if (!slug) return;
    setTrackError("");
    const result = await groups.lookupGroupBySlug(slug);
    if (result) {
      groups.trackGroup(result[0]);
      setTrackSlugInput("");
    } else {
      setTrackError(`No group found with slug "${slug}"`);
    }
  };

  const handleEditName = async (groupId: number, name: string) => {
    try {
      await groups.editEntryName(groupId, name);
    } catch {
      // error handled by hook
    }
  };

  if (groups.joinedGroups.length === 0) return null;

  return (
    <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
      <h2 className="text-lg font-semibold text-text-primary mb-1">
        Your Groups
      </h2>
      <p className="text-sm text-text-muted mb-4">
        Groups you have joined or created.
      </p>

      {groups.error && (
        <div className="mb-3 text-sm text-red-400 bg-red-900/20 rounded-lg px-3 py-2">
          {groups.error}
        </div>
      )}

      <div className="space-y-3 mb-4">
        {groups.joinedGroups.map((joined) => (
          <JoinedGroupCard
            key={joined.groupId}
            {...joined}
            isBeforeDeadline={isBeforeDeadline}
            isLoading={groups.isLoading}
            onEditName={handleEditName}
            onLeave={(id) => groups.leaveGroup(id)}
          />
        ))}
      </div>

      <div className="pt-3 border-t border-border">
        <h4 className="text-xs text-text-tertiary mb-1">
          Already a member? Track by slug
        </h4>
        <div className="flex gap-2 max-w-md">
          <input
            type="text"
            value={trackSlugInput}
            onChange={(v) => {
              setTrackSlugInput(v.target.value);
              setTrackError("");
            }}
            placeholder="group-slug"
            className="w-full max-w-md px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
          />
          <button
            onClick={handleTrackBySlug}
            disabled={!trackSlugInput.trim()}
            className="px-3 py-1.5 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:text-text-primary transition-colors whitespace-nowrap"
          >
            Track
          </button>
        </div>
        {trackError && (
          <p className="text-xs text-red-400 mt-1">{trackError}</p>
        )}
      </div>
    </div>
  );
}
