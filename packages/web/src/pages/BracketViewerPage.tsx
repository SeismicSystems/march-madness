import { useParams, Link } from "react-router-dom";
import { useMemo } from "react";

import { validateBracket, scoreBracketPartial } from "@march-madness/client";

import { BracketView } from "../components/BracketView";
import { useEntries } from "../hooks/useEntries";
import { useReadOnlyBracket } from "../hooks/useReadOnlyBracket";
import { useTeamProbs } from "../hooks/useTeamProbs";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import { truncateAddress } from "../lib/tournament";

export function BracketViewerPage() {
  const { address } = useParams<{ address: string }>();
  const { entries, loading: entriesLoading } = useEntries();
  const { status: tournamentStatus } = useTournamentStatus();
  const { teamProbs } = useTeamProbs();

  const entry =
    address && entries
      ? (entries[address.toLowerCase()] ?? entries[address])
      : null;
  const bracketHex =
    entry?.bracket && validateBracket(entry.bracket)
      ? (entry.bracket as `0x${string}`)
      : null;

  const games = useReadOnlyBracket(bracketHex);

  const score = useMemo(() => {
    if (!bracketHex || !tournamentStatus) return null;
    return scoreBracketPartial(bracketHex, tournamentStatus);
  }, [bracketHex, tournamentStatus]);

  const getGamesForRound = useMemo(() => {
    return (round: number) => games.filter((g) => g.round === round);
  }, [games]);

  if (entriesLoading) {
    return (
      <div className="text-center py-12 text-text-muted">
        Loading bracket...
      </div>
    );
  }

  if (!entry || !bracketHex) {
    return (
      <div className="text-center py-12">
        <p className="text-text-muted mb-4">
          {address
            ? `No bracket found for ${truncateAddress(address)}`
            : "Invalid address"}
        </p>
        <Link
          to="/leaderboard"
          className="text-accent hover:text-accent-hover text-sm"
        >
          Back to leaderboard
        </Link>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <div className="flex items-center gap-2">
            <Link
              to="/leaderboard"
              className="text-text-muted hover:text-text-primary text-sm transition-colors"
            >
              Leaderboard
            </Link>
            <span className="text-text-muted">/</span>
            <h2 className="text-lg font-bold text-text-primary">
              {entry.name || truncateAddress(address!)}
            </h2>
          </div>
          {entry.name && (
            <p className="text-xs text-text-muted font-mono mt-0.5">
              {address}
            </p>
          )}
        </div>
        {score && (
          <div className="text-right">
            <div className="text-2xl font-bold text-text-primary">
              {score.current}
            </div>
            <div className="text-xs text-text-muted">
              max {score.maxPossible}
            </div>
          </div>
        )}
      </div>

      <BracketView
        games={games}
        getGamesForRound={getGamesForRound}
        onPick={() => {}}
        disabled
        tournamentStatus={tournamentStatus ?? undefined}
        teamProbs={teamProbs}
      />
    </div>
  );
}
