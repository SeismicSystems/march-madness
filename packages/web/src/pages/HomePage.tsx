import { usePrivy } from "@privy-io/react-auth";
import { useRef, useState } from "react";


import { BracketView } from "../components/BracketView";
import { DeadlineCountdown } from "../components/DeadlineCountdown";
import { FaucetBanner } from "../components/FaucetBanner";
import { GroupsSection } from "../components/GroupsSection";
import { MirrorsSection } from "../components/MirrorsSection";
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

  // Easter egg: double-click to unlock hex input, type/paste a bracket to auto-fill
  const [hexOpen, setHexOpen] = useState(false);
  const [hexInput, setHexInput] = useState("");
  const [hexError, setHexError] = useState<string | null>(null);
  const hexRef = useRef<HTMLInputElement>(null);
  const tryLoadHex = (raw: string) => {
    // Strip whitespace and any non-hex garbage that might come from copy-paste
    const cleaned = raw.replace(/[^0-9a-fA-Fx]/g, "");
    // Need 0x + exactly 16 hex chars
    if (!/^0x[0-9a-fA-F]{16}$/.test(cleaned)) {
      setHexError(null); // Not a complete hex yet, no error
      return false;
    }
    // Check sentinel bit — warn but still load
    const firstNibble = parseInt(cleaned[2], 16);
    if (firstNibble < 8) {
      setHexError("Missing sentinel bit — picks loaded but bracket is invalid for on-chain submission");
    } else {
      setHexError(null);
    }
    bracket.loadFromHex(cleaned as `0x${string}`);
    setHexInput("");
    setHexOpen(false);
    return true;
  };
  const handleHexChange = (value: string) => {
    setHexInput(value);
    tryLoadHex(value);
  };
  const handleHexPaste = (e: React.ClipboardEvent) => {
    const pasted = e.clipboardData.getData("text");
    if (tryLoadHex(pasted)) {
      e.preventDefault();
    }
  };

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
          <>
            <button
              onClick={bracket.resetPicks}
              className="px-3 py-2 text-xs rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
            >
              Reset Picks
            </button>
            {hexOpen ? (
              <input
                ref={hexRef}
                type="text"
                value={hexInput}
                onChange={(e) => handleHexChange(e.target.value)}
                onPaste={handleHexPaste}
                onBlur={() => {
                  // Check the DOM value directly to avoid stale closure
                  if (!hexRef.current?.value) setHexOpen(false);
                }}
                placeholder="0x..."
                spellCheck={false}
                autoFocus
                className="w-40 px-2 py-1.5 text-xs font-mono rounded-lg bg-bg-tertiary border border-accent/50 text-text-primary placeholder-text-muted/50 focus:outline-none transition-colors"
              />
            ) : (
              <span
                onDoubleClick={() => setHexOpen(true)}
                className={`px-2 py-1.5 text-xs font-mono select-none cursor-default ${bracket.encodedBracket ? "text-text-muted" : "text-text-muted/30"}`}
              >
                {bracket.encodedBracket ?? "0x"}
              </span>
            )}
          </>
        )}
      </div>

      {hexError && (
        <div className="mb-2 px-3 py-1.5 text-xs text-yellow-400 bg-yellow-400/10 border border-yellow-400/30 rounded-lg">
          {hexError}
        </div>
      )}

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

      {/* Mirrors — tucked away, only shows if user has tracked mirrors */}
      <div className="mb-6 sm:mb-8">
        <MirrorsSection />
      </div>

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
