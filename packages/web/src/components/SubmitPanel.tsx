import { useCallback, useEffect, useRef, useState } from "react";

import { useIsMobile } from "../hooks/useIsMobile";
import type { UseContractReturn } from "../hooks/useContract";
import type { UseBracketReturn } from "../hooks/useBracket";
import { ConfirmDialog } from "./ConfirmDialog";

interface SubmitPanelProps {
  contract: UseContractReturn;
  bracket: UseBracketReturn;
  walletConnected: boolean;
  totalEntries: number | null;
  requiresChainSwitch: boolean;
  isSwitchingChain: boolean;
  requiredChainName: string;
  chainSwitchError: string | null;
  onSwitchChain: () => Promise<boolean>;
  onLoadBracket: () => Promise<void>;
}

export function SubmitPanel({
  contract,
  bracket,
  walletConnected,
  totalEntries,
  requiresChainSwitch,
  isSwitchingChain,
  requiredChainName,
  chainSwitchError,
  onSwitchChain,
  onLoadBracket,
}: SubmitPanelProps) {
  const {
    hasSubmitted,
    isLoading,
    isBracketLoading,
    error,
    existingBracket,
    entryFeeDisplay,
  } = contract;
  const displayError = chainSwitchError ?? error;
  const feeDisplay = entryFeeDisplay ?? "...";
  const { isComplete, pickCount, encodedBracket } = bracket;
  const [tag, setTag] = useState("");
  const [tagSaved, setTagSaved] = useState(false);
  const [submitSuccess, setSubmitSuccess] = useState(false);
  const isMobile = useIsMobile();

  const isLocked = !contract.isBeforeDeadline;

  // Reset picks
  const [resetOpen, setResetOpen] = useState(false);

  // Hex input
  const [hexOpen, setHexOpen] = useState(false);
  const [hexExpanded, setHexExpanded] = useState(false);
  const [hexCopied, setHexCopied] = useState(false);
  const [hexInput, setHexInput] = useState("");
  const [hexError, setHexError] = useState<string | null>(null);
  const hexRef = useRef<HTMLInputElement>(null);
  const expandRef = useRef<HTMLDivElement>(null);
  const collapseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const collapseHexControls = useCallback(() => {
    if (collapseTimer.current) {
      clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }
    setHexExpanded(false);
    setHexCopied(false);
  }, []);

  const scheduleCollapse = useCallback(() => {
    if (collapseTimer.current) clearTimeout(collapseTimer.current);
    collapseTimer.current = setTimeout(() => {
      setHexExpanded(false);
      setHexCopied(false);
      collapseTimer.current = null;
    }, 3000);
  }, []);

  useEffect(() => {
    return () => {
      if (collapseTimer.current) clearTimeout(collapseTimer.current);
    };
  }, []);

  useEffect(() => {
    if (!hexExpanded) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (
        expandRef.current &&
        !expandRef.current.contains(event.target as Node)
      ) {
        collapseHexControls();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [collapseHexControls, hexExpanded]);

  const handleCopy = async () => {
    if (!encodedBracket) return;
    await navigator.clipboard.writeText(encodedBracket);
    setHexCopied(true);
    setTimeout(() => {
      setHexCopied(false);
      setHexExpanded(false);
    }, 1200);
  };
  const tryLoadHex = (raw: string) => {
    const cleaned = raw.replace(/[^0-9a-fA-Fx]/g, "");
    if (!/^0x[0-9a-fA-F]{16}$/.test(cleaned)) {
      setHexError(null);
      return false;
    }
    const firstNibble = parseInt(cleaned[2], 16);
    if (firstNibble < 8) {
      const fixedNibble = (firstNibble | 0x8).toString(16);
      const fixed = `0x${fixedNibble}${cleaned.slice(3)}` as `0x${string}`;
      setHexError(
        `Pasted ${cleaned} — missing sentinel bit. Loaded ${fixed} (same picks, valid for submission).`,
      );
      bracket.loadFromHex(fixed);
    } else {
      setHexError(null);
      bracket.loadFromHex(cleaned as `0x${string}`);
    }
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
  const openHexEditor = () => {
    collapseHexControls();
    setHexOpen(true);
  };

  const handleSubmit = async () => {
    if (!encodedBracket) return;
    try {
      if (hasSubmitted) {
        await contract.updateBracket(encodedBracket);
      } else {
        await contract.submitBracket(encodedBracket);
      }
      setSubmitSuccess(true);
      setTimeout(() => setSubmitSuccess(false), 3000);
    } catch {
      // Error is handled by the hook
    }
  };

  const handleSetTag = async () => {
    if (!tag.trim()) return;
    try {
      await contract.setTag(tag.trim());
      setTagSaved(true);
      setTimeout(() => setTagSaved(false), 3000);
    } catch {
      // Error is handled by the hook
    }
  };

  const hexControl = (
    <div className="flex items-center shrink-0">
      {hexOpen ? (
        <input
          ref={hexRef}
          type="text"
          value={hexInput}
          onChange={(e) => handleHexChange(e.target.value)}
          onPaste={handleHexPaste}
          onBlur={() => {
            if (!hexRef.current?.value) setHexOpen(false);
          }}
          placeholder="0x..."
          spellCheck={false}
          autoFocus
          className="w-[7.5rem] sm:w-[9rem] px-2 py-2 text-[11px] font-mono rounded-lg bg-transparent border border-accent/50 text-text-primary placeholder-text-muted/50 focus:outline-none transition-colors"
        />
      ) : (
        <div
          ref={expandRef}
          className="flex items-center w-[12rem] sm:w-[13.5rem]"
        >
          <span
            onDoubleClick={() => {
              setHexExpanded((prev) => !prev);
              scheduleCollapse();
            }}
            className={`min-w-0 flex-1 truncate rounded-lg border border-border/70 bg-transparent px-2 py-2 text-[11px] font-mono select-none cursor-default ${encodedBracket ? "text-text-muted" : "text-text-muted/30"}`}
          >
            {encodedBracket ?? "0x"}
          </span>
          <div
            className={`ml-1 flex max-w-0 items-center gap-1 overflow-hidden transition-[max-width,opacity] duration-200 ease-out ${hexExpanded ? "max-w-[3.5rem] opacity-100" : "opacity-0 pointer-events-none"}`}
          >
            {hexCopied ? (
              <span className="w-full px-1 py-1 text-center text-[10px] text-green-400 whitespace-nowrap">
                Copied!
              </span>
            ) : (
              <button
                onClick={handleCopy}
                title="Copy hex"
                className="p-1 rounded hover:bg-bg-hover text-text-muted hover:text-text-primary transition-colors"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                  className="w-3.5 h-3.5"
                >
                  <path d="M7 3.5A1.5 1.5 0 0 1 8.5 2h3.879a1.5 1.5 0 0 1 1.06.44l3.122 3.12A1.5 1.5 0 0 1 17 6.622V12.5a1.5 1.5 0 0 1-1.5 1.5h-1v-3.379a3 3 0 0 0-.879-2.121L10.5 5.379A3 3 0 0 0 8.379 4.5H7v-1Z" />
                  <path d="M4.5 6A1.5 1.5 0 0 0 3 7.5v9A1.5 1.5 0 0 0 4.5 18h7a1.5 1.5 0 0 0 1.5-1.5v-5.879a1.5 1.5 0 0 0-.44-1.06L9.44 6.439A1.5 1.5 0 0 0 8.378 6H4.5Z" />
                </svg>
              </button>
            )}
            <button
              onClick={openHexEditor}
              title="Edit hex"
              className="p-1 rounded hover:bg-bg-hover text-text-muted hover:text-text-primary transition-colors"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
                className="w-3.5 h-3.5"
              >
                <path d="m5.433 13.917 1.262-3.155A4 4 0 0 1 7.58 9.42l6.92-6.918a2.121 2.121 0 0 1 3 3l-6.92 6.918c-.383.383-.84.685-1.343.886l-3.154 1.262a.5.5 0 0 1-.65-.65Z" />
                <path d="M3.5 5.75c0-.69.56-1.25 1.25-1.25H10A.75.75 0 0 0 10 3H4.75A2.75 2.75 0 0 0 2 5.75v9.5A2.75 2.75 0 0 0 4.75 18h9.5A2.75 2.75 0 0 0 17 15.25V10a.75.75 0 0 0-1.5 0v5.25c0 .69-.56 1.25-1.25 1.25h-9.5c-.69 0-1.25-.56-1.25-1.25v-9.5Z" />
              </svg>
            </button>
          </div>
        </div>
      )}
    </div>
  );

  if (isMobile) {
    return (
      <MobileSubmitPanel
        isComplete={isComplete}
        pickCount={pickCount}
        hasSubmitted={hasSubmitted}
        isLoading={isLoading}
        isBracketLoading={isBracketLoading}
        error={error}
        chainSwitchError={chainSwitchError}
        existingBracket={existingBracket}
        isLocked={isLocked}
        submitSuccess={submitSuccess}
        tag={tag}
        tagSaved={tagSaved}
        walletConnected={walletConnected}
        requiresChainSwitch={requiresChainSwitch}
        isSwitchingChain={isSwitchingChain}
        requiredChainName={requiredChainName}
        entryFeeDisplay={entryFeeDisplay}
        resetOpen={resetOpen}
        onSwitchChain={onSwitchChain}
        onSubmit={handleSubmit}
        onLoadBracket={onLoadBracket}
        onSetTag={handleSetTag}
        onTagChange={setTag}
        onResetOpen={setResetOpen}
        onResetPicks={bracket.resetPicks}
      />
    );
  }

  // Desktop: compact horizontal bar
  return (
    <div className="bg-bg-secondary border border-border rounded-xl px-4 py-3 space-y-2">
      {requiresChainSwitch && (
        <div className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
          Switch your wallet to {requiredChainName} to submit. If the network
          is missing, we&apos;ll prompt MetaMask to add it first.
        </div>
      )}
      <div className="flex items-center gap-4">
        {/* Progress */}
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-xs text-text-secondary whitespace-nowrap">
            Picks
          </span>
          <span
            className={`text-sm font-mono font-semibold ${isComplete ? "text-success" : "text-text-primary"}`}
          >
            {pickCount}/63
          </span>
          <div className="w-24 bg-bg-tertiary rounded-full h-1.5">
            <div
              className={`h-1.5 rounded-full transition-all duration-300 ${isComplete ? "bg-success" : "bg-accent"}`}
              style={{ width: `${(pickCount / 63) * 100}%` }}
            />
          </div>
        </div>

        {/* Status badges */}
        {hasSubmitted && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-success/20 text-success border border-success/30 whitespace-nowrap">
            Submitted
          </span>
        )}
        {isLocked && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-danger/20 text-danger border border-danger/30 whitespace-nowrap">
            Locked
          </span>
        )}

        {/* Entry fee (before first submission) */}
        {!hasSubmitted && !isLocked && (
          <div className="flex items-center gap-4 whitespace-nowrap">
            <span className="text-xs text-text-muted">
              Entry:{" "}
              <span className="font-semibold text-text-primary">
                {feeDisplay}
              </span>
            </span>
            {totalEntries != null && (
              <span className="text-xs text-text-muted">
                Brackets{" "}
                <span className="font-semibold text-text-primary">
                  {totalEntries}
                </span>
              </span>
            )}
          </div>
        )}

        <div className="flex-1" />

        {/* Load existing bracket */}
        {hasSubmitted && !existingBracket && (
          <button
            onClick={onLoadBracket}
            disabled={isBracketLoading}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all border whitespace-nowrap ${
              isBracketLoading
                ? "bg-bg-tertiary text-text-muted cursor-wait border-border"
                : "bg-bg-tertiary text-text-primary border-border hover:bg-bg-hover hover:border-accent/50"
            }`}
          >
            {isBracketLoading ? "Loading..." : "Load my bracket"}
          </button>
        )}

        {/* Tag input (after submission) */}
        {hasSubmitted && !isLocked && (
          <div className="flex items-center gap-1.5">
            <input
              type="text"
              value={tag}
              onChange={(e) => setTag(e.target.value)}
              placeholder="Display name"
              maxLength={32}
              className="w-52 px-2 py-1.5 text-xs rounded-lg bg-bg-tertiary border border-border text-text-primary placeholder-text-muted focus:outline-none focus:border-accent"
            />
            <button
              onClick={handleSetTag}
              disabled={!tag.trim() || isLoading}
              className="px-2 py-1.5 text-xs rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
            >
              {tagSaved ? "Saved!" : "Set Tag"}
            </button>
          </div>
        )}

        {/* Divider between tag and submit */}
        {hasSubmitted && !isLocked && <div className="w-px h-6 bg-border" />}

        {/* Reset + Submit buttons */}
        {!isLocked && (
          <div className="flex items-center gap-2">
            {hexControl}
            <button
              onClick={() => setResetOpen(true)}
              className="px-3 py-2 rounded-lg text-xs font-medium bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors whitespace-nowrap"
            >
              Reset Picks
            </button>
            <ConfirmDialog
              open={resetOpen}
              onClose={() => setResetOpen(false)}
              onConfirm={bracket.resetPicks}
              title="Reset Picks?"
              description={
                hasSubmitted
                  ? 'This will clear all 63 picks. You can re-load your on-chain submission by clicking "Load bracket".'
                  : "This will clear all 63 picks. This can't be undone."
              }
              confirmLabel="Reset"
              cancelLabel="Cancel"
              danger
            />
            <button
              onClick={
                requiresChainSwitch ? () => void onSwitchChain() : handleSubmit
              }
              disabled={
                requiresChainSwitch
                  ? isSwitchingChain
                  : !isComplete || isLoading || !walletConnected
              }
              className={`px-6 py-2 rounded-lg font-semibold text-sm transition-all whitespace-nowrap ${
                requiresChainSwitch
                  ? "bg-warning text-black hover:brightness-110 ring-2 ring-warning/30 cursor-pointer"
                  : !isComplete || !walletConnected
                  ? "bg-bg-tertiary text-text-muted cursor-not-allowed border border-border"
                  : isLoading
                    ? "bg-accent/50 text-white cursor-wait"
                    : submitSuccess
                      ? "bg-success text-white ring-2 ring-success/30 cursor-pointer"
                      : "bg-accent text-white hover:bg-accent-hover ring-2 ring-accent/30 cursor-pointer"
              }`}
            >
              {requiresChainSwitch
                ? isSwitchingChain
                  ? `Switching to ${requiredChainName}...`
                  : `Switch to ${requiredChainName}`
                : isLoading
                ? "Submitting..."
                : submitSuccess
                  ? "Success!"
                  : !walletConnected
                    ? "Connect wallet"
                    : !isComplete
                      ? `${63 - pickCount} picks left`
                      : hasSubmitted
                      ? "Update Bracket"
                        : `Submit (${feeDisplay})`}
            </button>
          </div>
        )}
      </div>

      {/* Hex error */}
      {hexError && (
        <div className="px-3 py-1.5 text-xs text-warning bg-warning/10 border border-warning/30 rounded-lg">
          {hexError}
        </div>
      )}

      {/* Error row */}
      {displayError && (
        <div
          onClick={() => {
            navigator.clipboard.writeText(displayError);
          }}
          className="bg-danger/10 border border-danger/30 rounded-lg px-3 py-2 text-xs text-danger max-h-20 overflow-y-auto break-words cursor-pointer active:bg-danger/20"
        >
          <span className="font-semibold">Error (tap to copy):</span> {displayError}
        </div>
      )}
    </div>
  );
}

/* ── Mobile: keep the existing card layout ──────────── */

function MobileSubmitPanel({
  isComplete,
  pickCount,
  hasSubmitted,
  isLoading,
  isBracketLoading,
  error,
  chainSwitchError,
  existingBracket,
  isLocked,
  submitSuccess,
  tag,
  tagSaved,
  walletConnected,
  requiresChainSwitch,
  isSwitchingChain,
  requiredChainName,
  entryFeeDisplay,
  resetOpen,
  onSwitchChain,
  onSubmit,
  onLoadBracket,
  onSetTag,
  onTagChange,
  onResetOpen,
  onResetPicks,
}: {
  isComplete: boolean;
  pickCount: number;
  hasSubmitted: boolean;
  isLoading: boolean;
  isBracketLoading: boolean;
  error: string | null;
  chainSwitchError: string | null;
  existingBracket: `0x${string}` | null;
  isLocked: boolean;
  submitSuccess: boolean;
  tag: string;
  tagSaved: boolean;
  walletConnected: boolean;
  requiresChainSwitch: boolean;
  isSwitchingChain: boolean;
  requiredChainName: string;
  entryFeeDisplay: string | null;
  resetOpen: boolean;
  onSwitchChain: () => Promise<boolean>;
  onSubmit: () => Promise<void>;
  onLoadBracket: () => Promise<void>;
  onSetTag: () => Promise<void>;
  onTagChange: (v: string) => void;
  onResetOpen: (v: boolean) => void;
  onResetPicks: () => void;
}) {
  const feeDisplay = entryFeeDisplay ?? "...";
  const displayError = chainSwitchError ?? error;
  return (
    <div className="bg-bg-secondary border border-border rounded-xl p-4 space-y-4">
      {requiresChainSwitch && (
        <div className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
          Switch your wallet to {requiredChainName} to submit. If MetaMask
          doesn&apos;t have it yet, we&apos;ll ask to add it.
        </div>
      )}

      {/* Progress */}
      <div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-text-secondary">Picks made</span>
          <span
            className={`font-mono ${isComplete ? "text-success" : "text-text-primary"}`}
          >
            {pickCount}/63
          </span>
        </div>
        <div className="w-full bg-bg-tertiary rounded-full h-2">
          <div
            className={`h-2 rounded-full transition-all duration-300 ${isComplete ? "bg-success" : "bg-accent"}`}
            style={{ width: `${(pickCount / 63) * 100}%` }}
          />
        </div>
      </div>

      {/* Status badges */}
      <div className="flex flex-wrap gap-2">
        {hasSubmitted && (
          <span className="text-xs px-2 py-1 rounded-full bg-success/20 text-success border border-success/30">
            Bracket submitted
          </span>
        )}
        {isLocked && (
          <span className="text-xs px-2 py-1 rounded-full bg-danger/20 text-danger border border-danger/30">
            Brackets locked
          </span>
        )}
      </div>

      {/* Load existing bracket */}
      {hasSubmitted && !existingBracket && (
        <button
          onClick={onLoadBracket}
          disabled={isBracketLoading}
          className={`w-full py-2.5 rounded-lg text-sm font-medium transition-all border ${
            isBracketLoading
              ? "bg-bg-tertiary text-text-muted cursor-wait border-border"
              : "bg-bg-tertiary text-text-primary border-border hover:bg-bg-hover hover:border-accent/50"
          }`}
        >
          {isBracketLoading ? "Loading..." : "Load my bracket"}
        </button>
      )}

      {/* Entry fee */}
      {!hasSubmitted && !isLocked && (
        <div className="bg-bg-tertiary rounded-lg p-3 border border-border">
          <div className="text-xs text-text-muted mb-1">Entry fee</div>
          <div className="text-lg font-bold text-text-primary">
            {feeDisplay}
          </div>
          <div className="text-xs text-text-muted mt-1">
            Prize pool split equally among highest-scoring brackets
          </div>
        </div>
      )}

      {/* Submit button */}
      {!isLocked && (
        <button
          onClick={requiresChainSwitch ? () => void onSwitchChain() : onSubmit}
          disabled={
            requiresChainSwitch
              ? isSwitchingChain
              : !isComplete || isLoading || !walletConnected
          }
          className={`w-full py-3 rounded-lg font-semibold text-sm transition-all ${
            requiresChainSwitch
              ? "bg-warning text-black cursor-pointer"
              : !isComplete || !walletConnected
              ? "bg-bg-tertiary text-text-muted cursor-not-allowed border border-border"
              : isLoading
                ? "bg-accent/50 text-white cursor-wait"
                : submitSuccess
                  ? "bg-success text-white"
                  : "bg-accent text-white hover:bg-accent-hover"
          }`}
        >
          {requiresChainSwitch
            ? isSwitchingChain
              ? `Switching to ${requiredChainName}...`
              : `Switch to ${requiredChainName}`
            : isLoading
              ? "Submitting..."
              : submitSuccess
                ? "Success!"
                : !walletConnected
                  ? "Connect wallet to submit"
                  : !isComplete
                    ? `Complete your bracket (${63 - pickCount} picks remaining)`
                    : hasSubmitted
                      ? "Update Bracket"
                      : `Submit Bracket (${feeDisplay})`}
        </button>
      )}

      {!isLocked && (
        <div>
          <button
            onClick={() => onResetOpen(true)}
            className="w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors"
          >
            Reset bracket
          </button>
          <ConfirmDialog
            open={resetOpen}
            onClose={() => onResetOpen(false)}
            onConfirm={onResetPicks}
            title="Reset Picks?"
            description={
              hasSubmitted
                ? 'This will clear all 63 picks. You can re-load your on-chain submission by clicking "Load bracket".'
                : "This will clear all 63 picks. This can't be undone."
            }
            confirmLabel="Reset"
            cancelLabel="Cancel"
            danger
          />
        </div>
      )}

      {/* Tag input */}
      {hasSubmitted && !isLocked && (
        <div className="space-y-2">
          <label className="text-xs text-text-muted">
            Display name (optional)
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={tag}
              onChange={(e) => onTagChange(e.target.value)}
              placeholder="Enter a display name"
              maxLength={32}
              className="flex-1 px-3 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-primary placeholder-text-muted focus:outline-none focus:border-accent"
            />
            <button
              onClick={onSetTag}
              disabled={!tag.trim() || isLoading}
              className="px-4 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {tagSaved ? "Saved!" : "Set Tag"}
            </button>
          </div>
        </div>
      )}

      {/* Error display */}
      {displayError && (
        <div
          onClick={() => {
            navigator.clipboard.writeText(displayError);
          }}
          className="bg-danger/10 border border-danger/30 rounded-lg p-3 text-xs text-danger max-h-40 overflow-y-auto break-words cursor-pointer active:bg-danger/20"
        >
          <div className="font-semibold mb-1 text-sm">Error (tap to copy)</div>
          {displayError}
        </div>
      )}

    </div>
  );
}
