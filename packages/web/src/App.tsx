import { usePrivy } from "@privy-io/react-auth";

import { BracketView } from "./components/BracketView";
import { DeadlineCountdown } from "./components/DeadlineCountdown";
import { FaucetBanner } from "./components/FaucetBanner";
import { Header } from "./components/Header";
import { SubmitPanel } from "./components/SubmitPanel";
import { useBracket } from "./hooks/useBracket";
import { useContract } from "./hooks/useContract";

export default function App() {
  const { authenticated } = usePrivy();
  const contract = useContract();
  const bracket = useBracket(contract.walletAddress);

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
    <div className="min-h-screen bg-bg-primary">
      <Header entryCount={contract.entryCount} />

      <main className="max-w-[1800px] mx-auto px-2 sm:px-4 py-4 sm:py-6">
        {/* Faucet banner when connected with 0 balance */}
        {showFaucetBanner && (
          <FaucetBanner address={contract.walletAddress!} />
        )}

        {/* Top bar: countdown + reset */}
        <div className="flex items-center gap-2 sm:gap-4 mb-4">
          <DeadlineCountdown />
          <button
            onClick={bracket.resetPicks}
            className="px-3 py-2 text-xs rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
          >
            Reset Picks
          </button>
        </div>

        {/* Submit panel — full width horizontal bar on desktop */}
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

        {/* Bracket */}
        <BracketView
          games={bracket.games}
          getGamesForRound={bracket.getGamesForRound}
          onPick={bracket.makePick}
          disabled={isLocked}
        />
      </main>
    </div>
  );
}
