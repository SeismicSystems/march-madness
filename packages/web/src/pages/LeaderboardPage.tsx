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
import { useEntries } from "../hooks/useEntries";
import { useForecasts } from "../hooks/useForecasts";
import { useGroupForecasts } from "../hooks/useGroupForecasts";
import { useGroupMembers } from "../hooks/useGroupMembers";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import {
  displayName,
  getAllTeamsInBracketOrder,
  truncateAddress,
} from "../lib/tournament";

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
  const navigate = useNavigate();
  const { entries, loading: entriesLoading } = useEntries();
  const { status, loading: statusLoading } = useTournamentStatus();
  const { forecasts: globalForecasts } = useForecasts();
  const { forecasts: groupForecasts } = useGroupForecasts(slug);
  const forecasts = slug ? groupForecasts : globalForecasts;
  const {
    members: groupMembers,
    groupName,
    loading: groupLoading,
    error: groupError,
    notFound: groupNotFound,
  } = useGroupMembers(slug);

  const hasForecasts =
    forecasts !== null && Object.keys(forecasts).length > 0;

  const rows = useMemo((): LeaderboardRow[] => {
    if (!entries) return [];

    const out: LeaderboardRow[] = [];
    for (const [address, entry] of Object.entries(entries)) {
      if (groupMembers && !groupMembers.has(address.toLowerCase())) continue;

      const bracketHex =
        entry.bracket && validateBracket(entry.bracket)
          ? (entry.bracket as `0x${string}`)
          : null;
      const score =
        bracketHex && status ? scoreBracketPartial(bracketHex, status) : null;
      const forecast = bracketHex
        ? forecasts?.[address] ?? forecasts?.[address.toLowerCase()]
        : undefined;

      out.push({
        id: address,
        label: entry.name || truncateAddress(address),
        sublabel: entry.name ? truncateAddress(address) : undefined,
        bracket: bracketHex,
        score,
        championName: bracketHex ? getChampionName(bracketHex) : null,
        forecast,
        sortLabel: (entry.name ?? address).toLowerCase(),
      });
    }

    return out;
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

  return (
    <LeaderboardTable
      rows={rows}
      title={slug ? `${groupName ?? slug} Leaderboard` : "Leaderboard"}
      backLink={slug ? { to: "/leaderboard", label: "All" } : undefined}
      status={status}
      hasForecasts={hasForecasts}
      onRowClick={(row) => navigate(`/bracket/${row.id}`)}
    />
  );
}
