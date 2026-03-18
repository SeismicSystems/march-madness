import { useMemo } from "react";
import { Link, useParams } from "react-router-dom";

import {
  decodeBracket,
  scoreBracketPartial,
  validateBracket,
} from "@march-madness/client";

import {
  LeaderboardCard,
  type LeaderboardEntry,
} from "../components/LeaderboardCard";
import { useEntries } from "../hooks/useEntries";
import { useForecasts } from "../hooks/useForecasts";
import { useGroupMembers } from "../hooks/useGroupMembers";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import { displayName, getAllTeamsInBracketOrder } from "../lib/tournament";

const teamNames = getAllTeamsInBracketOrder().map((t) => displayName(t));

function getChampionName(hex: `0x${string}`): string {
  try {
    return decodeBracket(hex, teamNames).champion;
  } catch {
    return "?";
  }
}

export function LeaderboardPage() {
  const { slug } = useParams<{ slug?: string }>();
  const { entries, loading: entriesLoading } = useEntries();
  const { status, loading: statusLoading } = useTournamentStatus();
  const { forecasts } = useForecasts();
  const {
    members: groupMembers,
    groupName,
    loading: groupLoading,
  } = useGroupMembers(slug);

  const hasForecasts = forecasts !== null && Object.keys(forecasts).length > 0;

  const leaderboard = useMemo((): LeaderboardEntry[] => {
    if (!entries || !status) return [];

    const scored: LeaderboardEntry[] = [];
    for (const [address, entry] of Object.entries(entries)) {
      // Filter by group membership when viewing a group leaderboard.
      if (groupMembers && !groupMembers.has(address.toLowerCase())) continue;
      if (!entry.bracket || !validateBracket(entry.bracket)) continue;
      const bracketHex = entry.bracket as `0x${string}`;
      const score = scoreBracketPartial(bracketHex, status);
      const forecast =
        forecasts?.[address] ?? forecasts?.[address.toLowerCase()];
      scored.push({
        address,
        tag: entry.name,
        bracket: bracketHex,
        score,
        championName: bracketHex ? getChampionName(bracketHex) : null,
        forecast,
        sortLabel: (entry.name ?? address).toLowerCase(),
      });
    }

    scored.sort((a, b) => {
      if (b.score.current !== a.score.current)
        return b.score.current - a.score.current;
      return b.score.maxPossible - a.score.maxPossible;
    });

    return scored;
  }, [entries, status, forecasts, groupMembers]);

  const loading = entriesLoading || statusLoading || groupLoading;

    return rows;
  }, [entries, forecasts, groupMembers, status]);

  if (slug && (groupNotFound || groupError)) {
    return (
      <div className="max-w-xl mx-auto text-center py-12">
        <h2 className="text-lg font-bold text-text-primary mb-2">
          Group leaderboard unavailable
        </h2>
        <p className="text-text-muted mb-4">
          {groupError ?? `Group "/${slug}" not found.`}
        </p>
        <Link
          to="/groups"
          className="text-sm text-accent hover:text-accent-hover transition-colors"
        >
          Back to groups
        </Link>
      </div>
    );
  }

  const loading = entriesLoading || statusLoading || groupLoading;

  if (loading) {
    return (
      <div className="text-center py-12 text-text-muted">
        Loading leaderboard...
      </div>
    );
  }

  if (leaderboard.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted">No entries found.</div>
    );
  }

  const decidedCount =
    status?.games.filter((g) => g.status === "final").length ?? 0;
  const liveCount =
    status?.games.filter((g) => g.status === "live").length ?? 0;

  return (
    <div className="w-full mx-auto">
      <div className="flex items-center justify-between mb-4  mx-2 md:mx-auto md:w-3/4">
        <div className="flex items-center gap-2 ">
          {slug && (
            <Link
              to="/leaderboard"
              className="text-xs text-accent hover:text-accent-hover transition-colors"
            >
              All
            </Link>
          )}
          <h2 className="text-lg font-bold text-text-primary">
            {slug ? `${groupName ?? slug} Leaderboard` : "Leaderboard"}
          </h2>
        </div>
        <div className="flex gap-3 text-xs text-text-muted">
          <span>{decidedCount}/63 games decided</span>
          {liveCount > 0 && (
            <span className="text-green-400">{liveCount} live</span>
          )}
          <h2 className="text-lg font-bold text-text-primary">
            {slug ? `${groupName ?? slug} Leaderboard` : "Leaderboard"}
          </h2>
        </div>
        {status && (
          <div className="flex gap-3 text-xs text-text-muted">
            <span>{decidedCount}/63 games decided</span>
            {liveCount > 0 && (
              <span className="text-green-400">{liveCount} live</span>
            )}
          </div>
        )}
      </div>

      <div className="flex flex-col gap-3 sm:w-3/4 sm:mx-auto">
        {leaderboard.map((entry, i) => (
          <LeaderboardCard
            key={entry.address}
            entry={entry}
            rank={i + 1}
            hasForecasts={hasForecasts}
          />
        ))}
      </div>
    </div>
  );
}
