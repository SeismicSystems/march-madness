import { useMemo } from "react";
import { Link, useParams } from "react-router-dom";

import {
  decodeBracket,
  scoreBracketPartial,
  validateBracket,
} from "@march-madness/client";
import type { BracketForecast, PartialScore } from "@march-madness/client";

import { useEntries } from "../hooks/useEntries";
import { useForecasts } from "../hooks/useForecasts";
import { useGroupMembers } from "../hooks/useGroupMembers";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import {
  displayName,
  getAllTeamsInBracketOrder,
  truncateAddress,
} from "../lib/tournament";

interface LeaderboardEntry {
  address: string;
  tag?: string;
  bracket: `0x${string}` | null;
  score: PartialScore | null;
  championName: string | null;
  forecast?: BracketForecast;
  sortLabel: string;
}

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
    error: groupError,
    notFound: groupNotFound,
  } = useGroupMembers(slug);

  const hasForecasts = forecasts !== null && Object.keys(forecasts).length > 0;

  const leaderboard = useMemo((): LeaderboardEntry[] => {
    if (!entries) return [];

    const rows: LeaderboardEntry[] = [];
    for (const [address, entry] of Object.entries(entries)) {
      if (groupMembers && !groupMembers.has(address.toLowerCase())) continue;

      const bracketHex =
        entry.bracket && validateBracket(entry.bracket)
          ? (entry.bracket as `0x${string}`)
          : null;
      const score =
        bracketHex && status ? scoreBracketPartial(bracketHex, status) : null;
      const forecast = bracketHex
        ? (forecasts?.[address] ?? forecasts?.[address.toLowerCase()])
        : undefined;

      rows.push({
        address,
        tag: entry.name,
        bracket: bracketHex,
        score,
        championName: bracketHex ? getChampionName(bracketHex) : null,
        forecast,
        sortLabel: (entry.name ?? address).toLowerCase(),
      });
    }

    rows.sort((a, b) => {
      if (a.score && b.score) {
        if (b.score.current !== a.score.current) {
          return b.score.current - a.score.current;
        }
        if (b.score.maxPossible !== a.score.maxPossible) {
          return b.score.maxPossible - a.score.maxPossible;
        }
      } else if (a.score) {
        return -1;
      } else if (b.score) {
        return 1;
      }

      if (a.sortLabel !== b.sortLabel) {
        return a.sortLabel.localeCompare(b.sortLabel);
      }

      return a.address.localeCompare(b.address);
    });

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
    <div>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
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
        {status && (
          <div className="flex gap-3 text-xs text-text-muted">
            <span>{decidedCount}/63 games decided</span>
            {liveCount > 0 && (
              <span className="text-green-400">{liveCount} live</span>
            )}
          </div>
        )}
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border text-left text-text-muted">
              <th className="py-2 pr-2 w-10">#</th>
              <th className="py-2 px-2">Player</th>
              <th className="py-2 px-2 text-right">Score</th>
              <th className="py-2 px-2 text-right hidden sm:table-cell">Max</th>
              {hasForecasts && (
                <>
                  <th className="py-2 px-2 text-right hidden sm:table-cell">
                    E[Score]
                  </th>
                  <th className="py-2 px-2 text-right hidden md:table-cell">
                    P(Win)
                  </th>
                </>
              )}
              <th className="py-2 px-2 hidden lg:table-cell">Champion</th>
              <th className="py-2 pl-2 w-16"></th>
            </tr>
          </thead>
          <tbody>
            {leaderboard.map((entry, i) => (
              <LeaderboardRow
                key={entry.address}
                entry={entry}
                rank={i + 1}
                hasForecasts={hasForecasts}
              />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function LeaderboardRow({
  entry,
  rank,
  hasForecasts,
}: {
  entry: LeaderboardEntry;
  rank: number;
  hasForecasts: boolean;
}) {
  return (
    <tr className="border-b border-border/50 hover:bg-bg-hover/50 transition-colors">
      <td className="py-2.5 pr-2 text-text-muted font-mono">{rank}</td>
      <td className="py-2.5 px-2">
        <div className="flex flex-col">
          {entry.tag ? (
            <>
              <span className="text-text-primary font-medium">{entry.tag}</span>
              <span className="text-[10px] text-text-muted font-mono">
                {truncateAddress(entry.address)}
              </span>
            </>
          ) : (
            <span className="text-text-primary font-mono">
              {truncateAddress(entry.address)}
            </span>
          )}
        </div>
      </td>
      <td className="py-2.5 px-2 text-right">
        {entry.score ? (
          <>
            <span className="text-text-primary font-bold">
              {entry.score.current}
            </span>
            <span className="text-text-muted sm:hidden">
              /{entry.score.maxPossible}
            </span>
          </>
        ) : (
          <span className="text-text-muted">—</span>
        )}
      </td>
      <td className="py-2.5 px-2 text-right text-text-muted hidden sm:table-cell">
        {entry.score ? entry.score.maxPossible : "—"}
      </td>
      {hasForecasts && (
        <>
          <td className="py-2.5 px-2 text-right text-text-secondary hidden sm:table-cell">
            {entry.forecast ? entry.forecast.expectedScore.toFixed(1) : "—"}
          </td>
          <td className="py-2.5 px-2 text-right hidden md:table-cell">
            {entry.forecast ? (
              <span
                className={
                  entry.forecast.winProbability > 0.1
                    ? "text-green-400 font-medium"
                    : "text-text-muted"
                }
              >
                {(entry.forecast.winProbability * 100).toFixed(1)}%
              </span>
            ) : (
              "—"
            )}
          </td>
        </>
      )}
      <td className="py-2.5 px-2 text-text-secondary hidden lg:table-cell">
        {entry.championName ?? "—"}
      </td>
      <td className="py-2.5 pl-2">
        {entry.bracket ? (
          <Link
            to={`/bracket/${entry.address}`}
            className="text-xs text-accent hover:text-accent-hover transition-colors"
          >
            View
          </Link>
        ) : (
          <span className="text-xs text-text-muted">—</span>
        )}
      </td>
    </tr>
  );
}
