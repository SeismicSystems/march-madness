import { useState } from "react";
import { formatEther } from "viem";

import type { GroupWinningsState } from "../hooks/useGroupWinningsState";
import type { WinningsState } from "../hooks/useWinningsState";

type WinningsBannerProps =
  | { type: "main"; state: WinningsState }
  | { type: "group"; state: GroupWinningsState };

function truncateHash(hash: `0x${string}`): string {
  return `${hash.slice(0, 10)}…${hash.slice(-6)}`;
}

export function WinningsBanner({ type, state }: WinningsBannerProps) {
  const [txHash, setTxHash] = useState<`0x${string}` | null>(null);
  const [scoreTxHash, setScoreTxHash] = useState<`0x${string}` | null>(null);

  const {
    resultsPostedAt,
    isWinner,
    hasCollected,
    canClaim,
    payoutAmount,
    collectWinnings,
    isCollecting,
    error,
  } = state;

  const canScore = type === "main" ? (state as WinningsState).canScore : false;
  const scoreBracket =
    type === "main" ? (state as WinningsState).scoreBracket : null;
  const isScoring =
    type === "main" ? (state as WinningsState).isScoring : false;

  // Nothing to show until results are posted
  if (!resultsPostedAt || resultsPostedAt === 0n) return null;

  const handleCollect = async () => {
    try {
      const hash = await collectWinnings();
      setTxHash(hash);
    } catch {
      // error already set in hook state
    }
  };

  const handleCollectEntryFee = async () => {
    if (type !== "main") return;
    try {
      const hash = await (state as WinningsState).collectEntryFee();
      setTxHash(hash);
    } catch {
      // error already set in hook state
    }
  };

  // ── Already collected ────────────────────────────────────────────
  if (isWinner && hasCollected) {
    return (
      <div className="bg-success/10 border border-success/30 rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
        <div className="text-sm font-semibold text-success">
          Winnings collected
        </div>
        {txHash && (
          <p className="text-xs text-text-muted mt-1 font-mono">
            tx: {truncateHash(txHash)}
          </p>
        )}
      </div>
    );
  }

  // ── Claim winnings ───────────────────────────────────────────────
  if (canClaim) {
    const payoutLabel =
      payoutAmount !== null
        ? `${formatEther(payoutAmount)} ETH`
        : "your winnings";
    return (
      <div className="bg-warning/10 border border-warning/30 rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
        <div className="text-sm font-semibold text-warning mb-2">
          You won! Claim {payoutLabel}
        </div>
        <p className="text-xs sm:text-sm text-text-secondary mb-3">
          The scoring window has closed. Collect your prize now.
        </p>
        {error && <p className="text-xs text-red-400 mb-2">{error}</p>}
        {txHash ? (
          <p className="text-xs text-success font-mono">
            Transaction sent: {truncateHash(txHash)}
          </p>
        ) : (
          <button
            type="button"
            onClick={handleCollect}
            disabled={isCollecting}
            className="px-4 py-1.5 rounded-lg bg-warning text-black font-semibold text-xs sm:text-sm hover:bg-warning/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isCollecting ? "Collecting..." : `Claim ${payoutLabel}`}
          </button>
        )}
      </div>
    );
  }

  // ── No-contest: collect entry fee refund (main pool only) ────────
  if (type === "main") {
    const mainState = state as WinningsState;
    if (mainState.canClaimEntryFee) {
      return (
        <div className="bg-warning/10 border border-warning/30 rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
          <div className="text-sm font-semibold text-warning mb-2">
            Results were never posted
          </div>
          <p className="text-xs sm:text-sm text-text-secondary mb-3">
            The owner did not submit results within the required window. You can
            reclaim your entry fee.
          </p>
          {error && <p className="text-xs text-red-400 mb-2">{error}</p>}
          {txHash ? (
            <p className="text-xs text-success font-mono">
              Transaction sent: {truncateHash(txHash)}
            </p>
          ) : (
            <button
              type="button"
              onClick={handleCollectEntryFee}
              disabled={isCollecting}
              className="px-4 py-1.5 rounded-lg bg-warning text-black font-semibold text-xs sm:text-sm hover:bg-warning/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isCollecting ? "Collecting..." : "Claim Entry Fee Refund"}
            </button>
          )}
        </div>
      );
    }
  }

  // ── Score submitted confirmation ─────────────────────────────────
  if (scoreTxHash && !canScore) {
    return (
      <div className="bg-success/10 border border-success/30 rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
        <div className="text-sm font-semibold text-success">
          Score submitted
        </div>
        <p className="text-xs text-text-muted mt-1 font-mono">
          tx: {truncateHash(scoreTxHash)}
        </p>
      </div>
    );
  }

  // ── Score my bracket (main pool only) ───────────────────────────
  if (canScore) {
    const handleScore = async () => {
      if (!scoreBracket) return;
      try {
        const hash = await scoreBracket();
        setScoreTxHash(hash);
      } catch {
        // error already set in hook state
      }
    };
    const mainState = state as WinningsState;
    return (
      <div className="bg-bg-secondary border border-border rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
        <div className="text-sm font-semibold text-text-primary mb-1">
          Results posted — score your bracket
        </div>
        <p className="text-xs sm:text-sm text-text-muted mb-3">
          Submit your final score on-chain. Winners can claim their prize after
          the scoring window closes.
        </p>
        {mainState.error && (
          <p className="text-xs text-red-400 mb-2">{mainState.error}</p>
        )}
        <button
          type="button"
          onClick={handleScore}
          disabled={isScoring}
          className="px-4 py-1.5 rounded-lg bg-accent text-bg-primary font-semibold text-xs sm:text-sm hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isScoring ? "Scoring..." : "Score My Bracket"}
        </button>
      </div>
    );
  }

  // ── Group only: score all members ────────────────────────────────
  if (type === "group") {
    const groupState = state as GroupWinningsState;
    if (!groupState.allScored && groupState.payouts?.numWinners === 0) {
      return (
        <div className="bg-bg-secondary border border-border rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
          <div className="text-sm font-semibold text-text-primary mb-1">
            Members not yet scored
          </div>
          <p className="text-xs sm:text-sm text-text-muted mb-3">
            Score all members to determine the winner and unlock prize
            collection.
          </p>
          {groupState.error && (
            <p className="text-xs text-red-400 mb-2">{groupState.error}</p>
          )}
          <button
            type="button"
            onClick={() => groupState.scoreAllMembers()}
            disabled={groupState.isScoringAll}
            className="px-4 py-1.5 rounded-lg bg-accent text-bg-primary font-semibold text-xs sm:text-sm hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {groupState.isScoringAll ? "Scoring..." : "Score All Members"}
          </button>
        </div>
      );
    }
  }

  return null;
}
