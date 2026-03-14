import { usePrivy } from "@privy-io/react-auth";

import { BracketView } from "./components/BracketView";
import { DeadlineCountdown } from "./components/DeadlineCountdown";
import { Header } from "./components/Header";
import { Scoreboard } from "./components/Scoreboard";
import { SubmitPanel } from "./components/SubmitPanel";
import { useBracket } from "./hooks/useBracket";
import { useContract } from "./hooks/useContract";

export default function App() {
  const { authenticated } = usePrivy();
  const bracket = useBracket();
  const contract = useContract();

  // Load existing bracket if user has one
  // (only do this once when existingBracket changes)
  const loadedRef = { current: false };
  if (contract.existingBracket && !loadedRef.current) {
    bracket.loadFromHex(contract.existingBracket);
    loadedRef.current = true;
  }

  const isLocked = !contract.isBeforeDeadline;

  return (
    <div className="min-h-screen bg-bg-primary">
      <Header entryCount={contract.entryCount} />

      <main className="max-w-[1800px] mx-auto px-4 py-6">
        {/* Top bar: countdown + submit panel */}
        <div className="flex flex-col lg:flex-row gap-4 mb-8">
          <div className="flex-1 flex items-start gap-4">
            <DeadlineCountdown />
            <div className="flex items-center gap-2">
              <button
                onClick={bracket.resetPicks}
                className="px-3 py-2 text-xs rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
              >
                Reset Picks
              </button>
            </div>
          </div>
          <div className="w-full lg:w-80">
            <SubmitPanel
              isComplete={bracket.isComplete}
              pickCount={bracket.pickCount}
              hasSubmitted={contract.hasSubmitted}
              isLoading={contract.isLoading}
              error={contract.error}
              encodedBracket={bracket.encodedBracket}
              onSubmit={contract.submitBracket}
              onUpdate={contract.updateBracket}
              onSetTag={contract.setTag}
              walletConnected={authenticated}
            />
          </div>
        </div>

        {/* Bracket */}
        <BracketView
          games={bracket.games}
          getGamesForRound={bracket.getGamesForRound}
          onPick={bracket.makePick}
          disabled={isLocked}
        />

        {/* Scoreboard placeholder */}
        <div className="mt-12">
          <Scoreboard entryCount={contract.entryCount} />
        </div>
      </main>
    </div>
  );
}
