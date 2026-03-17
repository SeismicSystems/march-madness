import { usePrivy } from "@privy-io/react-auth";

import { BracketView } from "../components/BracketView";
import { DeadlineCountdown } from "../components/DeadlineCountdown";
import { FaucetBanner } from "../components/FaucetBanner";
import { SubmitPanel } from "../components/SubmitPanel";
import { useBracket } from "../hooks/useBracket";
import { useContract } from "../hooks/useContract";
import { useStats } from "../hooks/useStats";
import { useTournamentStatus } from "../hooks/useTournamentStatus";

export function HomePage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const bracket = useBracket(contract.walletAddress);
  const { status: tournamentStatus } = useTournamentStatus();
  const { totalEntries, loading: statsLoading, error: statsError } = useStats();

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
      {showFaucetBanner && <FaucetBanner address={contract.walletAddress!} />}

      <div className="mb-6 sm:mb-8">
        <SubmitPanel
          contract={contract}
          bracket={bracket}
          walletConnected={authenticated && !!contract.walletAddress}
          onLoadBracket={handleLoadBracket}
        />
      </div>

      <div className="flex flex-wrap items-center justify-center gap-2 sm:gap-4 mb-6 sm:mb-8">
        <DeadlineCountdown deadline={contract.submissionDeadline} />
        {!statsLoading && !statsError && totalEntries != null && (
          <div className="rounded-lg px-4 py-2 text-center bg-bg-tertiary border border-border">
            <div className="text-xs text-text-muted mb-1">
              Brackets submitted
            </div>
            <div className="font-mono font-bold text-sm text-text-primary">
              {totalEntries}
            </div>
          </div>
        )}
      </div>

      <BracketView
        games={bracket.games}
        getGamesForRound={bracket.getGamesForRound}
        onPick={bracket.makePick}
        disabled={isLocked}
        tournamentStatus={
          isLocked && tournamentStatus ? tournamentStatus : undefined
        }
      />
    </>
  );
}
