import { useState } from "react";

import { ENTRY_FEE_DISPLAY, SUBMISSION_DEADLINE } from "../lib/constants";

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

  return (
    <div className="bg-bg-secondary border border-border rounded-xl p-4 sm:p-6 space-y-4">
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
            className={`h-2 rounded-full transition-all duration-300 ${
              isComplete ? "bg-success" : "bg-accent"
            }`}
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

      {/* Load existing bracket button (only if submitted but not yet loaded) */}
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

      {/* Entry fee notice */}
      {!hasSubmitted && !isLocked && (
        <div className="bg-bg-tertiary rounded-lg p-3 border border-border">
          <div className="text-xs text-text-muted mb-1">Entry fee</div>
          <div className="text-lg font-bold text-text-primary">
            {ENTRY_FEE_DISPLAY}
          </div>
          <div className="text-xs text-text-muted mt-1">
            Prize pool split equally among highest-scoring brackets
          </div>
        </div>
      )}

      {/* Submit / Update button */}
      {!isLocked && (
        <button
          onClick={handleSubmit}
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

      {/* Tag input (only after submission) */}
      {hasSubmitted && !isLocked && (
        <div className="space-y-2">
          <label className="text-xs text-text-muted">
            Display name (optional)
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={tag}
              onChange={(e) => setTag(e.target.value)}
              placeholder="Enter a display name"
              maxLength={32}
              className="flex-1 px-3 py-2 text-sm rounded-lg bg-bg-tertiary border border-border text-text-primary placeholder-text-muted focus:outline-none focus:border-accent"
            />
            <button
              onClick={handleSetTag}
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
        <div className="bg-danger/10 border border-danger/30 rounded-lg p-3 text-sm text-danger max-h-32 overflow-y-auto break-words">
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
