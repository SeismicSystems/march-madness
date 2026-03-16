import { usePrivy } from "@privy-io/react-auth";
import { useCallback, useEffect, useRef, useState } from "react";


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

  // Easter egg: double-click to fan out copy/edit icons, edit opens hex input
  const [hexOpen, setHexOpen] = useState(false);
  const [hexExpanded, setHexExpanded] = useState(false);
  const [hexCopied, setHexCopied] = useState(false);
  const [hexInput, setHexInput] = useState("");
  const [hexError, setHexError] = useState<string | null>(null);
  const hexRef = useRef<HTMLInputElement>(null);
  const expandRef = useRef<HTMLDivElement>(null);
  const collapseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Auto-collapse after 3s of no interaction
  const scheduleCollapse = useCallback(() => {
    if (collapseTimer.current) clearTimeout(collapseTimer.current);
    collapseTimer.current = setTimeout(() => {
      setHexExpanded(false);
      setHexCopied(false);
    }, 3000);
  }, []);

  // Click-outside to collapse
  useEffect(() => {
    if (!hexExpanded) return;
    const handler = (e: MouseEvent) => {
      if (expandRef.current && !expandRef.current.contains(e.target as Node)) {
        setHexExpanded(false);
        setHexCopied(false);
        if (collapseTimer.current) clearTimeout(collapseTimer.current);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [hexExpanded]);

  const handleCopy = async () => {
    if (!bracket.encodedBracket) return;
    await navigator.clipboard.writeText(bracket.encodedBracket);
    setHexCopied(true);
    setTimeout(() => {
      setHexCopied(false);
      setHexExpanded(false);
    }, 1200);
  };
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
              <div ref={expandRef} className="relative flex items-center">
                <span
                  onDoubleClick={() => {
                    setHexExpanded((prev) => !prev);
                    scheduleCollapse();
                  }}
                  className={`px-2 py-1.5 text-xs font-mono select-none cursor-default ${bracket.encodedBracket ? "text-text-muted" : "text-text-muted/30"}`}
                >
                  {bracket.encodedBracket ?? "0x"}
                </span>
                {/* Fan-out action icons */}
                <div
                  className={`flex items-center gap-1 overflow-hidden transition-all duration-200 ease-out ${hexExpanded ? "max-w-[120px] opacity-100" : "max-w-0 opacity-0"}`}
                >
                  {hexCopied ? (
                    <span className="px-1.5 py-1 text-[10px] text-green-400 whitespace-nowrap">
                      Copied!
                    </span>
                  ) : (
                    <button
                      onClick={handleCopy}
                      title="Copy hex"
                      className="p-1 rounded hover:bg-bg-hover text-text-muted hover:text-text-primary transition-colors"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-3.5 h-3.5">
                        <path d="M7 3.5A1.5 1.5 0 0 1 8.5 2h3.879a1.5 1.5 0 0 1 1.06.44l3.122 3.12A1.5 1.5 0 0 1 17 6.622V12.5a1.5 1.5 0 0 1-1.5 1.5h-1v-3.379a3 3 0 0 0-.879-2.121L10.5 5.379A3 3 0 0 0 8.379 4.5H7v-1Z" />
                        <path d="M4.5 6A1.5 1.5 0 0 0 3 7.5v9A1.5 1.5 0 0 0 4.5 18h7a1.5 1.5 0 0 0 1.5-1.5v-5.879a1.5 1.5 0 0 0-.44-1.06L9.44 6.439A1.5 1.5 0 0 0 8.378 6H4.5Z" />
                      </svg>
                    </button>
                  )}
                  <button
                    onClick={() => {
                      setHexExpanded(false);
                      setHexOpen(true);
                      if (collapseTimer.current) clearTimeout(collapseTimer.current);
                    }}
                    title="Edit hex"
                    className="p-1 rounded hover:bg-bg-hover text-text-muted hover:text-text-primary transition-colors"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-3.5 h-3.5">
                      <path d="m5.433 13.917 1.262-3.155A4 4 0 0 1 7.58 9.42l6.92-6.918a2.121 2.121 0 0 1 3 3l-6.92 6.918c-.383.383-.84.685-1.343.886l-3.154 1.262a.5.5 0 0 1-.65-.65Z" />
                      <path d="M3.5 5.75c0-.69.56-1.25 1.25-1.25H10A.75.75 0 0 0 10 3H4.75A2.75 2.75 0 0 0 2 5.75v9.5A2.75 2.75 0 0 0 4.75 18h9.5A2.75 2.75 0 0 0 17 15.25V10a.75.75 0 0 0-1.5 0v5.25c0 .69-.56 1.25-1.25 1.25h-9.5c-.69 0-1.25-.56-1.25-1.25v-9.5Z" />
                    </svg>
                  </button>
                </div>
              </div>
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
