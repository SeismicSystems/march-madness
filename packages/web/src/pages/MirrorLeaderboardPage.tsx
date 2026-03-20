import { useMemo } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import {
  decodeBracket,
  scoreBracketPartial,
  validateBracket,
} from "@march-madness/client";

import {
  LeaderboardTable,
  type LeaderboardRow,
} from "../components/LeaderboardTable";
import { useMirror } from "../hooks/useMirror";
import { useMirrorForecasts } from "../hooks/useMirrorForecasts";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import {
  displayName,
  getAllTeamsInBracketOrder,
} from "../lib/tournament";

const teamNames = getAllTeamsInBracketOrder().map((t) => displayName(t));

function getChampionName(hex: `0x${string}`): string {
  try {
    return decodeBracket(hex, teamNames).champion;
  } catch {
    return "?";
  }
}

export function MirrorLeaderboardPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { mirror, entries: mirrorEntries, loading: mirrorLoading, notFound, error: mirrorError } = useMirror(id);
  const { status, loading: statusLoading } = useTournamentStatus();
  const { forecasts } = useMirrorForecasts(id);

  const hasForecasts =
    forecasts !== null && Object.keys(forecasts).length > 0;

  const rows = useMemo((): LeaderboardRow[] => {
    if (!mirrorEntries) return [];

    return mirrorEntries.map((entry) => {
      const bracketHex =
        entry.bracket && validateBracket(entry.bracket)
          ? (entry.bracket as `0x${string}`)
          : null;
      const score =
        bracketHex && status ? scoreBracketPartial(bracketHex, status) : null;
      const forecast = forecasts?.[entry.slug];

      return {
        id: entry.slug,
        label: entry.slug,
        bracket: bracketHex,
        score,
        championName: bracketHex ? getChampionName(bracketHex) : null,
        forecast,
        sortLabel: entry.slug.toLowerCase(),
      };
    });
  }, [mirrorEntries, forecasts, status]);

  if (notFound || mirrorError) {
    return (
      <div className="max-w-xl mx-auto text-center py-12">
        <h2 className="text-lg font-bold text-text-primary mb-2">
          Mirror not found
        </h2>
        <p className="text-text-muted mb-4">
          {mirrorError ?? `Mirror with ID "${id}" was not found.`}
        </p>
      </div>
    );
  }

  const loading = mirrorLoading || statusLoading;

  if (loading) {
    return (
      <div className="text-center py-12 text-text-muted">
        Loading leaderboard...
      </div>
    );
  }

  return (
    <>
      <div className="flex justify-end mx-2 md:mx-auto md:w-3/4 mb-1">
        <Link
          to={`/mirrors/id/${id}/ff`}
          className="text-xs text-accent hover:text-accent-hover transition-colors"
        >
          Final Four →
        </Link>
      </div>
      <LeaderboardTable
        rows={rows}
        title={mirror?.display_name ?? `Mirror ${id}`}
        status={status}
        hasForecasts={hasForecasts}
        onRowClick={(row) => navigate(`/mirrors/id/${id}/bracket/${row.id}`)}
        entryLabel="Entry"
      />
    </>
  );
}
