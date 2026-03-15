import { useState } from "react";

import { ENTRY_FEE_DISPLAY, SUBMISSION_DEADLINE } from "../lib/constants";
import { useIsMobile } from "../hooks/useIsMobile";

interface SubmitPanelProps {
  isComplete: boolean;
  pickCount: number;
  hasSubmitted: boolean;
  isLoading: boolean;
  isBracketLoading: boolean;
  error: string | null;
  encodedBracket: `0x${string}` | null;
  existingBracket: `0x${string}` | null;
  onSubmit: (bracket: `0x${string}`) => Promise<unknown>;
  onUpdate: (bracket: `0x${string}`) => Promise<unknown>;
  onSetTag: (tag: string) => Promise<unknown>;
  onLoadBracket: () => Promise<void>;
  walletConnected: boolean;
}

export function SubmitPanel({
  isComplete,
  pickCount,
  hasSubmitted,
  isLoading,
  isBracketLoading,
  error,
  encodedBracket,
  existingBracket,
  onSubmit,
  onUpdate,
  onSetTag,
  onLoadBracket,
  walletConnected,
}: SubmitPanelProps) {
  const [tag, setTag] = useState("");
  const [tagSaved, setTagSaved] = useState(false);
  const [submitSuccess, setSubmitSuccess] = useState(false);
  const isMobile = useIsMobile();

  const isLocked = Date.now() / 1000 >= SUBMISSION_DEADLINE;

  const handleSubmit = async () => {
    if (!encodedBracket) return;
    try {
      if (hasSubmitted) {
        await onUpdate(encodedBracket);
      } else {
        await onSubmit(encodedBracket);
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
      await onSetTag(tag.trim());
      setTagSaved(true);
      setTimeout(() => setTagSaved(false), 3000);
    } catch {
      // Error is handled by the hook
    }
  };

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
        onSubmit={handleSubmit}
        onLoadBracket={onLoadBracket}
        onSetTag={handleSetTag}
        onTagChange={setTag}
      />
    );
  }

  // Desktop: compact horizontal bar
  return (
    <div className="bg-bg-secondary border border-border rounded-xl px-4 py-3">
      <div className="flex items-center gap-4">
        {/* Progress */}
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-xs text-text-secondary whitespace-nowrap">Picks</span>
          <span className={`text-sm font-mono font-semibold ${isComplete ? "text-success" : "text-text-primary"}`}>
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
            Entry: <span className="font-semibold text-text-primary">{ENTRY_FEE_DISPLAY}</span>
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
              className="w-32 px-2 py-1.5 text-xs rounded-lg bg-bg-tertiary border border-border text-text-primary placeholder-text-muted focus:outline-none focus:border-accent"
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

        {/* Submit / Update button */}
        {!isLocked && (
          <button
            onClick={handleSubmit}
            disabled={!isComplete || isLoading || !walletConnected}
            className={`px-5 py-1.5 rounded-lg font-semibold text-xs transition-all whitespace-nowrap ${
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
                  ? "Connect wallet"
                  : !isComplete
                    ? `${63 - pickCount} picks left`
                    : hasSubmitted
                      ? "Update Bracket"
                      : `Submit (${ENTRY_FEE_DISPLAY})`}
          </button>
        )}
      </div>

      {/* Error row */}
      {error && (
        <div
          onClick={() => { navigator.clipboard.writeText(error); }}
          className="mt-2 bg-danger/10 border border-danger/30 rounded-lg px-3 py-2 text-xs text-danger max-h-20 overflow-y-auto break-words cursor-pointer active:bg-danger/20"
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
  onSubmit,
  onLoadBracket,
  onSetTag,
  onTagChange,
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
  onSubmit: () => Promise<void>;
  onLoadBracket: () => Promise<void>;
  onSetTag: () => Promise<void>;
  onTagChange: (v: string) => void;
}) {
  return (
    <div className="bg-bg-secondary border border-border rounded-xl p-4 space-y-4">
      {/* Progress */}
      <div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-text-secondary">Picks made</span>
          <span className={`font-mono ${isComplete ? "text-success" : "text-text-primary"}`}>
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
          <div className="text-lg font-bold text-text-primary">{ENTRY_FEE_DISPLAY}</div>
          <div className="text-xs text-text-muted mt-1">
            Prize pool split equally among highest-scoring brackets
          </div>
        </div>
      )}

      {/* Submit / Update button */}
      {!isLocked && (
        <button
          onClick={onSubmit}
          disabled={!isComplete || isLoading || !walletConnected}
          className={`w-full py-3 rounded-lg font-semibold text-sm transition-all ${
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
                    : `Submit Bracket (${ENTRY_FEE_DISPLAY})`}
        </button>
      )}

      {/* Tag input */}
      {hasSubmitted && !isLocked && (
        <div className="space-y-2">
          <label className="text-xs text-text-muted">Display name (optional)</label>
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
          onClick={() => { navigator.clipboard.writeText(error); }}
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
