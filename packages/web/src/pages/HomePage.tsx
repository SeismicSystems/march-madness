import { usePrivy } from "@privy-io/react-auth";

import { BracketView } from "../components/BracketView";
import { DeadlineCountdown } from "../components/DeadlineCountdown";
import { FaucetBanner } from "../components/FaucetBanner";
import { MobileBracketFooterControls } from "../components/MobileBracketFooterControls";
import { SubmitPanel } from "../components/SubmitPanel";
import { useBracket } from "../hooks/useBracket";
import { useContract } from "../hooks/useContract";
import { useIsMobile } from "../hooks/useIsMobile";
import { useStats } from "../hooks/useStats";
import { useTournamentStatus } from "../hooks/useTournamentStatus";

export function HomePage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const bracket = useBracket(contract.walletAddress);
  const { status: tournamentStatus } = useTournamentStatus();
  const { totalEntries, loading: statsLoading, error: statsError } = useStats();
  const isMobile = useIsMobile();

  const isLocked = !contract.isBeforeDeadline;
  const showEntries = !statsLoading && !statsError && totalEntries != null;

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
          totalEntries={showEntries ? totalEntries : null}
          onLoadBracket={handleLoadBracket}
        />
      </div>

      {isMobile ? (
        <div className="mb-4">
          <div className="rounded-lg px-3 py-2 bg-bg-tertiary border border-border flex items-center justify-between gap-3">
            <div className="text-xs text-text-muted">
              Brackets submitted{" "}
              <span className="font-mono font-semibold text-text-primary">
                {showEntries ? totalEntries : "..."}
              </span>
            </div>
            <DeadlineCountdown
              deadline={contract.submissionDeadline}
              compact
            />
          </div>
        </div>
      ) : (
        <div className="flex justify-center mb-6 sm:mb-8">
          <DeadlineCountdown deadline={contract.submissionDeadline} />
        </div>
      )}

      <BracketView
        games={bracket.games}
        getGamesForRound={bracket.getGamesForRound}
        onPick={bracket.makePick}
        disabled={isLocked}
        tournamentStatus={
          isLocked && tournamentStatus ? tournamentStatus : undefined
        }
      />

      {isMobile && (
        <MobileBracketFooterControls
          encodedBracket={bracket.encodedBracket}
          isLocked={isLocked}
          onLoadHex={bracket.loadFromHex}
        />
      )}
    </>
  );
}
