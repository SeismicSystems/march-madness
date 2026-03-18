import { useState } from "react";
import { formatEther } from "viem";
import { useSearchParams } from "react-router-dom";
import type { UseGroupsReturn } from "../hooks/useGroups";

interface PrivateJoinFormProps {
  groups: UseGroupsReturn;
  isBeforeDeadline: boolean;
  walletConnected: boolean;
  walletBalance: bigint | null;
  initialSlug?: string;
  initialPassphrase?: string;
  highlight?: boolean;
}

export function PrivateJoinForm({
  groups,
  isBeforeDeadline,
  walletConnected,
  walletBalance,
  initialSlug = "",
  initialPassphrase = "",
  highlight: initialHighlight = false,
}: PrivateJoinFormProps) {
  const [, setSearchParams] = useSearchParams();
  const [slugInput, setSlugInput] = useState(initialSlug);
  const [nameInput, setNameInput] = useState("");
  const [isPrivateJoin, setIsPrivateJoin] = useState(true);
  const [passphraseInput, setPassphraseInput] = useState(initialPassphrase);
  const [joinError, setJoinError] = useState<string | null>(null);
  const [highlighted, setHighlighted] = useState(initialHighlight);

  const handleJoin = async () => {
    if (!slugInput.trim() || !nameInput.trim()) return;
    setJoinError(null);

    try {
      const result = await groups.lookupGroupBySlug(slugInput.trim());
      if (!result) {
        setJoinError("Group not found");
        return;
      }

      const [groupId, groupData] = result;

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

      // Use the toggle state (not field contents or API-resolved group type)
      // to choose between joinGroup and joinGroupWithPassword
      if (isPrivateJoin) {
        await groups.joinGroupWithPassword(
          groupId,
          passphraseInput.trim(),
          nameInput.trim(),
          entryFee,
        );
      } else {
        await groups.joinGroup(groupId, nameInput.trim(), entryFee);
      }

      setSlugInput("");
      setNameInput("");
      setPassphraseInput("");
      setIsPrivateJoin(false);
      setHighlighted(false);
      // Clean invite params from URL
      setSearchParams({}, { replace: true });
    } catch (err) {
      setJoinError(err instanceof Error ? err.message : "Failed to join");
    }
  };

  if (!walletConnected || !isBeforeDeadline) return null;

  return (
    <div
      className={`rounded-xl p-4 sm:p-6 ${
        highlighted
          ? "bg-yellow-50 border-4 border-yellow-400 ring-4 ring-yellow-300/50 shadow-lg shadow-yellow-200/40"
          : "bg-bg-secondary border border-border"
      }`}
    >
      <h2 className="text-lg font-semibold text-text-primary mb-1">
        Join Group
      </h2>
      <p className="text-sm text-text-muted mb-4">
        Have an invite link or slug? Enter it here to join a group.
      </p>

      <div className="space-y-2 max-w-md">
        <input
          type="text"
          value={slugInput}
          onChange={(e) => setSlugInput(e.target.value)}
          placeholder="Group slug"
          className="w-full px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
        />
        <input
          type="text"
          value={nameInput}
          onChange={(e) => setNameInput(e.target.value)}
          placeholder="Your display name"
          className="w-full px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
        />
        <label className="flex items-center gap-2 text-sm text-text-secondary cursor-pointer">
          <input
            type="checkbox"
            checked={isPrivateJoin}
            onChange={(e) => setIsPrivateJoin(e.target.checked)}
            className="accent-accent"
          />
          Private group
        </label>
        {isPrivateJoin && (
          <input
            type="text"
            value={passphraseInput}
            onChange={(e) => setPassphraseInput(e.target.value)}
            placeholder="Passphrase"
            className="w-full px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
          />
        )}
        <div>
          <button
            onClick={handleJoin}
            disabled={
              groups.isLoading || !slugInput.trim() || !nameInput.trim()
            }
            className="px-4 py-1.5 text-sm rounded-lg bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors font-medium"
          >
            {groups.isLoading ? "Joining..." : "Join"}
          </button>
        </div>
      </div>
      {joinError && (
        <p className="text-xs text-red-400 mt-2">{joinError}</p>
      )}
    </div>
  );
}
