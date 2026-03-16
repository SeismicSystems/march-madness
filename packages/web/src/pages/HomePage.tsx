import { usePrivy } from "@privy-io/react-auth";

import { BracketView } from "../components/BracketView";
import { DeadlineCountdown } from "../components/DeadlineCountdown";
import { FaucetBanner } from "../components/FaucetBanner";
import { GroupsSection } from "../components/GroupsSection";
import { SubmitPanel } from "../components/SubmitPanel";
import { useBracket } from "../hooks/useBracket";
import { useContract } from "../hooks/useContract";
import { useGroups } from "../hooks/useGroups";
import { useTournamentStatus } from "../hooks/useTournamentStatus";

export function HomePage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const bracket = useBracket(contract.walletAddress);
  const groups = useGroups();
  const { status: tournamentStatus } = useTournamentStatus();

  const isLocked = !contract.isBeforeDeadline;
  const showFaucetBanner =
    authenticated &&
    contract.walletAddress &&
    contract.balance !== null &&
    contract.balance === 0n;

  const handleLoadBracket = async () => {
    const hex = await contract.loadMyBracket();
    if (hex) bracket.loadFromHex(hex);
  };

  return (
    <>
      {showFaucetBanner && (
        <FaucetBanner address={contract.walletAddress!} />
      )}

      <div className="flex items-center gap-2 sm:gap-4 mb-4">
        <DeadlineCountdown />
        {!isLocked && (
          <button
            onClick={bracket.resetPicks}
            className="px-3 py-2 text-xs rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
          >
            Reset Picks
          </button>
        )}
      </div>

      <div className="mb-6 sm:mb-8">
        <SubmitPanel
          isComplete={bracket.isComplete}
          pickCount={bracket.pickCount}
          hasSubmitted={contract.hasSubmitted}
          isLoading={contract.isLoading}
          isBracketLoading={contract.isBracketLoading}
          error={contract.error}
          encodedBracket={bracket.encodedBracket}
          existingBracket={contract.existingBracket}
          onSubmit={contract.submitBracket}
          onUpdate={contract.updateBracket}
          onSetTag={contract.setTag}
          onLoadBracket={handleLoadBracket}
          walletConnected={authenticated}
        />
      </div>

      {/* Groups section — prominent for both pre- and post-lock */}
      {groups.hasContract && (
        <div className="mb-6 sm:mb-8">
          <GroupsSection
            joinedGroups={groups.joinedGroups}
            isLoading={groups.isLoading}
            error={groups.error}
            isBeforeDeadline={contract.isBeforeDeadline}
            walletConnected={authenticated}
            onJoinGroup={groups.joinGroup}
            onLeaveGroup={groups.leaveGroup}
            onEditEntryName={groups.editEntryName}
            onLookupBySlug={groups.lookupGroupBySlug}
            onTrackGroup={groups.trackGroup}
          />
        </div>
      )}

      <BracketView
        games={bracket.games}
        getGamesForRound={bracket.getGamesForRound}
        onPick={bracket.makePick}
        disabled={isLocked}
        tournamentStatus={isLocked && tournamentStatus ? tournamentStatus : undefined}
      />
    </>
  );
}
