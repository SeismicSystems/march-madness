import { usePrivy } from "@privy-io/react-auth";

import { BracketView } from "../components/BracketView";
import { DeadlineCountdown } from "../components/DeadlineCountdown";
import { FaucetBanner } from "../components/FaucetBanner";
import { SubmitPanel } from "../components/SubmitPanel";
import { useBracket } from "../hooks/useBracket";
import { useContract } from "../hooks/useContract";
import { useRequiredChain } from "../hooks/useRequiredChain";
import { useStats } from "../hooks/useStats";
import { useTournamentStatus } from "../hooks/useTournamentStatus";

export function HomePage() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const requiredChain = useRequiredChain();
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
          totalEntries={!statsLoading && !statsError ? totalEntries : null}
          requiresChainSwitch={requiredChain.requiresChainSwitch}
          isSwitchingChain={requiredChain.isSwitchingChain}
          requiredChainName={requiredChain.requiredChain.name}
          chainSwitchError={requiredChain.switchChainError}
          onSwitchChain={requiredChain.switchToRequiredChain}
          onLoadBracket={handleLoadBracket}
        />
      </div>

      <div className="flex justify-center mb-6 sm:mb-8">
        <DeadlineCountdown deadline={contract.submissionDeadline} />
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
