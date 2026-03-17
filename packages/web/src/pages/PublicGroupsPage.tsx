import { usePrivy } from "@privy-io/react-auth";
import { Link } from "react-router-dom";

import { PublicGroupsList } from "../components/PublicGroupsList";
import { useContract } from "../hooks/useContract";
import { useGroups } from "../hooks/useGroups";
import { usePublicGroups } from "../hooks/usePublicGroups";

export function PublicGroupsPage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const groups = useGroups();
  const {
    publicGroups,
    isLoading: publicLoading,
    error: publicError,
  } = usePublicGroups();

  return (
    <div className="max-w-2xl mx-auto space-y-4">
      <div className="flex items-center gap-3">
        <Link
          to="/groups"
          className="text-sm text-accent hover:text-accent-hover transition-colors"
        >
          &larr; Back to Groups
        </Link>
      </div>

      <h1 className="text-xl font-bold text-text-primary">Public Groups</h1>

      {!groups.hasContract && (
        <div className="rounded-xl bg-bg-secondary border border-border p-4 sm:p-6">
          <p className="text-sm text-text-muted">
            Groups contract not deployed on this network.
          </p>
        </div>
      )}

      {groups.hasContract && (
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
    </div>
  );
}
