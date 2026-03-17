import { useRef, useState } from "react";

import { useIsMobile } from "../hooks/useIsMobile";
import type { UseContractReturn } from "../hooks/useContract";
import type { UseBracketReturn } from "../hooks/useBracket";
import { ConfirmDialog } from "./ConfirmDialog";

interface SubmitPanelProps {
  contract: UseContractReturn;
  bracket: UseBracketReturn;
  walletConnected: boolean;
  onLoadBracket: () => Promise<void>;
}

export function SubmitPanel({
  contract,
  bracket,
  walletConnected,
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
  const [hexCopied, setHexCopied] = useState(false);
  const [hexInput, setHexInput] = useState("");
  const [hexError, setHexError] = useState<string | null>(null);
  const hexRef = useRef<HTMLInputElement>(null);

  const handleCopy = async () => {
    if (!encodedBracket) return;
    await navigator.clipboard.writeText(encodedBracket);
    setHexCopied(true);
    setTimeout(() => setHexCopied(false), 1200);
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

  const hexInputBlock = (
    <div className="flex flex-col gap-0.5">
      <span className="text-[10px] uppercase tracking-wider text-text-muted">
        Enter contract hex
      </span>
      <div className="flex items-center gap-1">
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
            className="w-[10.5rem] px-3 py-1.5 text-xs font-mono rounded-lg bg-bg-primary border-2 border-accent/60 text-text-primary placeholder-text-muted/50 shadow-inner focus:outline-none focus:ring-1 focus:ring-accent/50 transition-colors"
          />
        ) : (
          <div
            onClick={() => setHexOpen(true)}
            className="w-[10.5rem] px-3 py-1.5 text-xs font-mono rounded-lg bg-bg-primary border-2 border-border text-text-muted cursor-text shadow-inner truncate hover:border-accent/40 transition-colors"
          >
            {encodedBracket || "0x..."}
          </div>
        )}
        {encodedBracket && (
          <button
            onClick={handleCopy}
            title="Copy hex"
            className="p-1 rounded hover:bg-bg-hover text-text-muted hover:text-text-primary transition-colors"
          >
            {hexCopied ? (
              <span className="text-[10px] text-green-400 whitespace-nowrap px-0.5">
                Copied!
              </span>
            ) : (
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
                className="w-3.5 h-3.5"
              >
                <path d="M7 3.5A1.5 1.5 0 0 1 8.5 2h3.879a1.5 1.5 0 0 1 1.06.44l3.122 3.12A1.5 1.5 0 0 1 17 6.622V12.5a1.5 1.5 0 0 1-1.5 1.5h-1v-3.379a3 3 0 0 0-.879-2.121L10.5 5.379A3 3 0 0 0 8.379 4.5H7v-1Z" />
                <path d="M4.5 6A1.5 1.5 0 0 0 3 7.5v9A1.5 1.5 0 0 0 4.5 18h7a1.5 1.5 0 0 0 1.5-1.5v-5.879a1.5 1.5 0 0 0-.44-1.06L9.44 6.439A1.5 1.5 0 0 0 8.378 6H4.5Z" />
              </svg>
            )}
          </button>
        )}
      </div>
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
        encodedBracket={encodedBracket}
        existingBracket={existingBracket}
        isLocked={isLocked}
        submitSuccess={submitSuccess}
        tag={tag}
        tagSaved={tagSaved}
        walletConnected={walletConnected}
        entryFeeDisplay={entryFeeDisplay}
        resetOpen={resetOpen}
        hexError={hexError}
        hexInputBlock={hexInputBlock}
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
          <span className="text-xs text-text-muted whitespace-nowrap">
            Entry:{" "}
            <span className="font-semibold text-text-primary">
              {feeDisplay}
            </span>
          </span>
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
              onClick={handleSubmit}
              disabled={!isComplete || isLoading || !walletConnected}
              className={`px-6 py-2 rounded-lg font-semibold text-sm transition-all whitespace-nowrap ${
                !isComplete || !walletConnected
                  ? "bg-bg-tertiary text-text-muted cursor-not-allowed border border-border"
                  : isLoading
                    ? "bg-accent/50 text-white cursor-wait"
                    : submitSuccess
                      ? "bg-success text-white ring-2 ring-success/30"
                      : "bg-accent text-white hover:bg-accent-hover ring-2 ring-accent/30"
              }`}
            >
              {isLoading
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

      {/* Hex input row */}
      {!isLocked && (
        <div className="flex items-end gap-3 pt-1 border-t border-border/50">
          {hexInputBlock}
        </div>
      )}

      {/* Hex error */}
      {hexError && (
        <div className="px-3 py-1.5 text-xs text-warning bg-warning/10 border border-warning/30 rounded-lg">
          {hexError}
        </div>
      )}

      {/* Error row */}
      {error && (
        <div
          onClick={() => {
            navigator.clipboard.writeText(error);
          }}
          className="bg-danger/10 border border-danger/30 rounded-lg px-3 py-2 text-xs text-danger max-h-20 overflow-y-auto break-words cursor-pointer active:bg-danger/20"
        >
          <span className="font-semibold">Error (tap to copy):</span> {error}
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
  encodedBracket,
  existingBracket,
  isLocked,
  submitSuccess,
  tag,
  tagSaved,
  walletConnected,
  entryFeeDisplay,
  resetOpen,
  hexError,
  hexInputBlock,
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
  encodedBracket: `0x${string}` | null;
  existingBracket: `0x${string}` | null;
  isLocked: boolean;
  submitSuccess: boolean;
  tag: string;
  tagSaved: boolean;
  walletConnected: boolean;
  entryFeeDisplay: string | null;
  resetOpen: boolean;
  hexError: string | null;
  hexInputBlock: React.ReactNode;
  onSubmit: () => Promise<void>;
  onLoadBracket: () => Promise<void>;
  onSetTag: () => Promise<void>;
  onTagChange: (v: string) => void;
  onResetOpen: (v: boolean) => void;
  onResetPicks: () => void;
}) {
  const feeDisplay = entryFeeDisplay ?? "...";
  return (
    <div className="bg-bg-secondary border border-border rounded-xl p-4 space-y-4">
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

      {/* Reset + Submit buttons */}
      {!isLocked && (
        <div className="flex gap-2">
          <button
            onClick={() => onResetOpen(true)}
            className="px-4 py-3 rounded-lg text-sm font-medium bg-bg-tertiary border border-border text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors whitespace-nowrap"
          >
            Reset Picks
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
          <button
            onClick={onSubmit}
            disabled={!isComplete || isLoading || !walletConnected}
            className={`flex-1 py-3 rounded-lg font-semibold text-sm transition-all ${
              !isComplete || !walletConnected
                ? "bg-bg-tertiary text-text-muted cursor-not-allowed border border-border"
                : isLoading
                  ? "bg-accent/50 text-white cursor-wait"
                  : submitSuccess
                    ? "bg-success text-white"
                    : "bg-accent text-white hover:bg-accent-hover"
            }`}
          >
            {isLoading
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
        </div>
      )}

      {/* Hex input row */}
      {!isLocked && (
        <div className="flex items-end gap-3 pt-2 border-t border-border/50">
          {hexInputBlock}
        </div>
      )}

      {/* Hex error */}
      {hexError && (
        <div className="px-3 py-1.5 text-xs text-warning bg-warning/10 border border-warning/30 rounded-lg">
          {hexError}
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
      {error && (
        <div
          onClick={() => {
            navigator.clipboard.writeText(error);
          }}
          className="bg-danger/10 border border-danger/30 rounded-lg p-3 text-xs text-danger max-h-40 overflow-y-auto break-words cursor-pointer active:bg-danger/20"
        >
          <div className="font-semibold mb-1 text-sm">Error (tap to copy)</div>
          {error}
        </div>
      )}

      {/* Encoded bracket preview */}
      {encodedBracket && (
        <div className="text-xs text-text-muted font-mono break-all bg-bg-tertiary rounded p-2">
          Bracket: {encodedBracket}
        </div>
      )}
    </div>
  );
}
