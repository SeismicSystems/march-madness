import { useMemo, useState } from "react";
import { usePrivy } from "@privy-io/react-auth";
import { useSearchParams } from "react-router-dom";

import { GroupsSection } from "../components/GroupsSection";
import { useContract } from "../hooks/useContract";
import { useGroups } from "../hooks/useGroups";

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
}

export function GroupsPage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const groups = useGroups();
  const [searchParams] = useSearchParams();

  // Read invite link params (e.g., /groups?slug=seismic-team&password=Quake100)
  const initialSlug = useMemo(() => searchParams.get("slug") ?? "", [searchParams]);
  const initialPassphrase = useMemo(() => searchParams.get("password") ?? "", [searchParams]);

  // Create group form state
  const [displayName, setDisplayName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugManual, setSlugManual] = useState(false);
  const [entryFee, setEntryFee] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [createSuccess, setCreateSuccess] = useState<string | null>(null);

  const handleDisplayNameChange = (value: string) => {
    setDisplayName(value);
    if (!slugManual) {
      setSlug(slugify(value));
    }
  };

  const handleSlugChange = (value: string) => {
    setSlugManual(true);
    setSlug(slugify(value));
  };

  const handleCreate = async () => {
    if (!displayName.trim() || !slug.trim()) return;
    setCreateError(null);
    setCreateSuccess(null);

    try {
      const fee = entryFee ? BigInt(Math.floor(parseFloat(entryFee) * 1e18)) : 0n;

      if (passphrase.trim()) {
        await groups.createGroupWithPassword(slug, displayName.trim(), fee, passphrase.trim());
      } else {
        await groups.createGroup(slug, displayName.trim(), fee);
      }

      setCreateSuccess(`Group "${displayName.trim()}" created! Look it up by slug "/${slug}" to track it.`);
      setDisplayName("");
      setSlug("");
      setSlugManual(false);
      setEntryFee("");
      setPassphrase("");
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : "Failed to create group");
    }
  };

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <h1 className="text-xl font-bold text-text-primary">Groups</h1>

      {/* Create Group Section */}
      {authenticated && contract.isBeforeDeadline && groups.hasContract && (
        <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
          <h2 className="text-lg font-semibold text-text-primary mb-1">
            Create a Group
          </h2>
          <p className="text-sm text-text-muted mb-4">
            Start a group for friends, leagues, or office pools. Share the slug
            so others can join.
          </p>

          <div className="space-y-3">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-text-secondary mb-1">
                  Display Name <span className="text-red-400">*</span>
                </label>
                <input
                  type="text"
                  value={displayName}
                  onChange={(e) => handleDisplayNameChange(e.target.value)}
                  placeholder="My March Madness Pool"
                  className="w-full px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
                />
              </div>
              <div>
                <label className="block text-xs text-text-secondary mb-1">
                  Slug <span className="text-red-400">*</span>
                </label>
                <div className="flex items-center">
                  <span className="text-sm text-text-muted mr-1">/</span>
                  <input
                    type="text"
                    value={slug}
                    onChange={(e) => handleSlugChange(e.target.value)}
                    placeholder="my-march-madness-pool"
                    className="w-full px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors font-mono"
                  />
                </div>
              </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-text-secondary mb-1">
                  Entry Fee (ETH)
                </label>
                <input
                  type="text"
                  value={entryFee}
                  onChange={(e) => setEntryFee(e.target.value)}
                  placeholder="0 (free)"
                  className="w-full px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors font-mono"
                />
              </div>
              <div>
                <label className="block text-xs text-text-secondary mb-1">
                  Passphrase{" "}
                  <span className="text-text-muted">(optional, makes group private)</span>
                </label>
                <input
                  type="text"
                  value={passphrase}
                  onChange={(e) => setPassphrase(e.target.value)}
                  placeholder="leave empty for public"
                  className="w-full px-3 py-2 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
                />
              </div>
            </div>

            <div className="flex items-center gap-3">
              <button
                onClick={handleCreate}
                disabled={
                  groups.isLoading || !displayName.trim() || !slug.trim()
                }
                className="px-4 py-2 text-sm rounded-lg bg-accent text-white hover:bg-accent-hover disabled:opacity-50 transition-colors font-medium"
              >
                {groups.isLoading ? "Creating..." : "Create Group"}
              </button>
              {passphrase.trim() && (
                <span className="text-xs text-text-muted">
                  Private group (passphrase required to join)
                </span>
              )}
            </div>

            {createError && (
              <p className="text-xs text-red-400">{createError}</p>
            )}
            {createSuccess && (
              <p className="text-xs text-green-400">{createSuccess}</p>
            )}
          </div>
        </div>
      )}

      {/* Existing groups section (join + list) */}
      {groups.hasContract && (
        <GroupsSection
          joinedGroups={groups.joinedGroups}
          isLoading={groups.isLoading}
          error={groups.error}
          isBeforeDeadline={contract.isBeforeDeadline}
          walletConnected={authenticated}
          walletBalance={contract.balance}
          onJoinGroup={groups.joinGroup}
          onJoinGroupWithPassword={groups.joinGroupWithPassword}
          onLeaveGroup={groups.leaveGroup}
          onEditEntryName={groups.editEntryName}
          onLookupBySlug={groups.lookupGroupBySlug}
          onLookupById={groups.lookupGroupById}
          onTrackGroup={groups.trackGroup}
          initialSlug={initialSlug}
          initialPassphrase={initialPassphrase}
        />
      )}

      {!groups.hasContract && (
        <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
          <p className="text-sm text-text-muted">
            Groups contract not deployed on this network.
          </p>
        </div>
      )}
    </div>
  );
}
