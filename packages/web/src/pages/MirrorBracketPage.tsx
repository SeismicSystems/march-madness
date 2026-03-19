import { useParams, Link } from "react-router-dom";
import { useMemo } from "react";

import { validateBracket, scoreBracketPartial } from "@march-madness/client";

import { BracketView } from "../components/BracketView";
import { useMirror } from "../hooks/useMirror";
import { useReadOnlyBracket } from "../hooks/useReadOnlyBracket";
import { useTeamProbs } from "../hooks/useTeamProbs";
import { useTournamentStatus } from "../hooks/useTournamentStatus";

export function MirrorBracketPage() {
  const { id, entrySlug } = useParams<{ id: string; entrySlug: string }>();
  const { mirror, entries: mirrorEntries, loading: mirrorLoading } = useMirror(id);
  const { status: tournamentStatus } = useTournamentStatus();
  const { teamProbs } = useTeamProbs();

  const entry = useMemo(() => {
    if (!mirrorEntries || !entrySlug) return null;
    return mirrorEntries.find((e) => e.slug === entrySlug) ?? null;
  }, [mirrorEntries, entrySlug]);

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

  if (mirrorLoading) {
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
          {entrySlug
            ? `No bracket found for "${entrySlug}"`
            : "Invalid entry"}
        </p>
        {id && (
          <Link
            to={`/mirrors/id/${id}`}
            className="text-accent hover:text-accent-hover text-sm"
          >
            Back to mirror
          </Link>
        )}
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <div className="flex items-center gap-2">
            <Link
              to={`/mirrors/id/${id}`}
              className="text-text-muted hover:text-text-primary text-sm transition-colors"
            >
              {mirror?.display_name ?? `Mirror ${id}`}
            </Link>
            <span className="text-text-muted">/</span>
            <h2 className="text-lg font-bold text-text-primary">
              {entrySlug}
            </h2>
          </div>
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
