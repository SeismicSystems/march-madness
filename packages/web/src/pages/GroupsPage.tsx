import { useMemo, useState } from "react";
import { usePrivy } from "@privy-io/react-auth";
import { Link, useSearchParams } from "react-router-dom";

import { GroupsSection } from "../components/GroupsSection";
import { PublicGroupsList } from "../components/PublicGroupsList";
import { PrivateJoinForm } from "../components/PrivateJoinForm";
import { useContract } from "../hooks/useContract";
import { useGroups } from "../hooks/useGroups";
import { usePublicGroups } from "../hooks/usePublicGroups";

const MAX_SLUG_LENGTH = 32; // Must match BracketGroups.sol MAX_SLUG_LENGTH

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, MAX_SLUG_LENGTH);
}

type Tab = "your-groups" | "public-groups" | "join-group" | "create-group";

// ---------- Create Group Form (shared between mobile + desktop) ----------

function CreateGroupForm({
  groups,
}: {
  groups: ReturnType<typeof useGroups>;
}) {
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

  const slugAtLimit = slug.length >= MAX_SLUG_LENGTH;

  const handleCreate = async () => {
    if (!displayName.trim() || !slug.trim()) return;
    setCreateError(null);
    setCreateSuccess(null);

    try {
      const fee = entryFee
        ? BigInt(Math.floor(parseFloat(entryFee) * 1e18))
        : 0n;

      if (passphrase.trim()) {
        await groups.createGroupWithPassword(
          slug,
          displayName.trim(),
          fee,
          passphrase.trim(),
        );
      } else {
        await groups.createGroup(slug, displayName.trim(), fee);
      }

      setCreateSuccess(
        `Group "${displayName.trim()}" created! Look it up by slug "/${slug}" to track it.`,
      );
      setDisplayName("");
      setSlug("");
      setSlugManual(false);
      setEntryFee("");
      setPassphrase("");
    } catch (err) {
      setCreateError(
        err instanceof Error ? err.message : "Failed to create group",
      );
    }
  };

  return (
    <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
      <h2 className="text-lg font-semibold text-text-primary mb-1">
        Create a Group
      </h2>
      <p className="text-sm text-text-muted mb-4">
        Start a group for friends, leagues, or office pools. Share the slug so
        others can join.
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
            {slugAtLimit && (
              <p className="text-xs text-yellow-400 mt-1">
                Slug truncated to {MAX_SLUG_LENGTH} characters (contract limit).
              </p>
            )}
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
              <span className="text-text-muted">
                (optional, makes group private)
              </span>
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
            disabled={groups.isLoading || !displayName.trim() || !slug.trim()}
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

        {createError && <p className="text-xs text-red-400">{createError}</p>}
        {createSuccess && (
          <p className="text-xs text-green-400">{createSuccess}</p>
        )}
      </div>
    </div>
  );
}

// ---------- Empty state for "Your Groups" ----------

function YourGroupsEmpty({ onSwitchTab }: { onSwitchTab: (tab: Tab) => void }) {
  return (
    <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6 text-center">
      <h2 className="text-lg font-semibold text-text-primary mb-2">
        Your Groups
      </h2>
      <p className="text-sm text-text-muted mb-4">
        You haven&apos;t joined any groups yet. Get started:
      </p>
      <div className="flex flex-col sm:flex-row items-center justify-center gap-2">
        <button
          onClick={() => onSwitchTab("public-groups")}
          className="px-4 py-2 text-sm rounded-lg bg-accent text-white hover:bg-accent-hover transition-colors font-medium"
        >
          Browse Public Groups
        </button>
        <button
          onClick={() => onSwitchTab("join-group")}
          className="px-4 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:text-text-primary transition-colors font-medium"
        >
          Join a Group
        </button>
        <button
          onClick={() => onSwitchTab("create-group")}
          className="px-4 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:text-text-primary transition-colors font-medium"
        >
          Create a Group
        </button>
      </div>
    </div>
  );
}

// ---------- Mobile Tab Layout (< md) ----------

function MobileLayout({
  groups,
  contract,
  authenticated,
  publicGroups,
  publicLoading,
  publicError,
  initialSlug,
  initialPassphrase,
}: {
  groups: ReturnType<typeof useGroups>;
  contract: ReturnType<typeof useContract>;
  authenticated: boolean;
  publicGroups: ReturnType<typeof usePublicGroups>["publicGroups"];
  publicLoading: boolean;
  publicError: string | null;
  initialSlug: string;
  initialPassphrase: string;
}) {
  const [activeTab, setActiveTab] = useState<Tab>("your-groups");

  const canCreateOrJoin = authenticated && contract.isBeforeDeadline;

  const tabs: { id: Tab; label: string }[] = [
    { id: "your-groups", label: "Your Groups" },
    { id: "public-groups", label: "Public Groups" },
    ...(canCreateOrJoin
      ? [
          { id: "join-group" as Tab, label: "Join Group" },
          { id: "create-group" as Tab, label: "Create Group" },
        ]
      : []),
  ];

  return (
    <div className="space-y-4">
      {/* Tab bar */}
      <div className="flex overflow-x-auto gap-1 border-b border-border pb-1 -mx-2 px-2">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-3 py-2 text-sm font-medium whitespace-nowrap rounded-t-lg transition-colors ${
              activeTab === tab.id
                ? "text-accent border-b-2 border-accent"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === "your-groups" &&
        (groups.joinedGroups.length > 0 ? (
          <GroupsSection
            groups={groups}
            isBeforeDeadline={contract.isBeforeDeadline}
            walletConnected={authenticated}
            walletBalance={contract.balance}
          />
        ) : (
          <YourGroupsEmpty onSwitchTab={setActiveTab} />
        ))}

      {activeTab === "public-groups" && (
        <PublicGroupsList
          publicGroups={publicGroups}
          isLoading={publicLoading}
          error={publicError}
          groups={groups}
          walletConnected={authenticated}
          isBeforeDeadline={contract.isBeforeDeadline}
          walletBalance={contract.balance}
        />
      )}

      {activeTab === "join-group" && canCreateOrJoin && (
        <PrivateJoinForm
          groups={groups}
          isBeforeDeadline={contract.isBeforeDeadline}
          walletConnected={authenticated}
          walletBalance={contract.balance}
          initialSlug={initialSlug}
          initialPassphrase={initialPassphrase}
        />
      )}

      {activeTab === "create-group" && canCreateOrJoin && (
        <CreateGroupForm groups={groups} />
      )}
    </div>
  );
}

// ---------- Desktop Hub Layout (>= md) ----------

function DesktopLayout({
  groups,
  contract,
  authenticated,
  initialSlug,
  initialPassphrase,
}: {
  groups: ReturnType<typeof useGroups>;
  contract: ReturnType<typeof useContract>;
  authenticated: boolean;
  initialSlug: string;
  initialPassphrase: string;
}) {
  const canCreateOrJoin = authenticated && contract.isBeforeDeadline;

  return (
    <div className="grid grid-cols-2 gap-6">
      {/* Left column: Create + Join stacked */}
      <div className="space-y-6">
        {canCreateOrJoin && <CreateGroupForm groups={groups} />}

        {canCreateOrJoin && (
          <PrivateJoinForm
            groups={groups}
            isBeforeDeadline={contract.isBeforeDeadline}
            walletConnected={authenticated}
            walletBalance={contract.balance}
            initialSlug={initialSlug}
            initialPassphrase={initialPassphrase}
          />
        )}

        {!canCreateOrJoin && (
          <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
            <p className="text-sm text-text-muted">
              {!authenticated
                ? "Connect your wallet to create or join groups."
                : "The submission deadline has passed."}
            </p>
          </div>
        )}
      </div>

      {/* Right column: Your Groups + Browse Public link */}
      <div className="space-y-6">
        {groups.joinedGroups.length > 0 ? (
          <GroupsSection
            groups={groups}
            isBeforeDeadline={contract.isBeforeDeadline}
            walletConnected={authenticated}
            walletBalance={contract.balance}
          />
        ) : (
          <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
            <h2 className="text-lg font-semibold text-text-primary mb-2">
              Your Groups
            </h2>
            <p className="text-sm text-text-muted">
              You haven&apos;t joined any groups yet. Browse public groups or
              join one using a slug.
            </p>
          </div>
        )}

        <Link
          to="/groups/public"
          className="flex items-center justify-center gap-2 rounded-xl bg-accent/10 border border-accent/30 p-4 text-accent hover:bg-accent/20 transition-colors font-medium"
        >
          Browse Public Groups &rarr;
        </Link>
      </div>
    </div>
  );
}

// ---------- Main GroupsPage ----------

export function GroupsPage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const groups = useGroups();
  const {
    publicGroups,
    isLoading: publicLoading,
    error: publicError,
  } = usePublicGroups();
  const [searchParams] = useSearchParams();

  const initialSlug = useMemo(
    () => searchParams.get("slug") ?? "",
    [searchParams],
  );
  const initialPassphrase = useMemo(
    () => searchParams.get("password") ?? "",
    [searchParams],
  );

  if (!groups.hasContract) {
    return (
      <div className="max-w-2xl mx-auto">
        <h1 className="text-xl font-bold text-text-primary mb-4">Groups</h1>
        <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
          <p className="text-sm text-text-muted">
            Groups contract not deployed on this network.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto">
      <h1 className="text-xl font-bold text-text-primary mb-4">Groups</h1>

      {/* Mobile: tab layout */}
      <div className="md:hidden">
        <MobileLayout
          groups={groups}
          contract={contract}
          authenticated={authenticated}
          publicGroups={publicGroups}
          publicLoading={publicLoading}
          publicError={publicError}
          initialSlug={initialSlug}
          initialPassphrase={initialPassphrase}
        />
      </div>

      {/* Desktop: hub layout */}
      <div className="hidden md:block">
        <DesktopLayout
          groups={groups}
          contract={contract}
          authenticated={authenticated}
          initialSlug={initialSlug}
          initialPassphrase={initialPassphrase}
        />
      </div>
    </div>
  );
}
