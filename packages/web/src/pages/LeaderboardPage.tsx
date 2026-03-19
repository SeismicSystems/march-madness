import { useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import {
  decodeBracket,
  scoreBracketPartial,
  validateBracket,
} from "@march-madness/client";

import type { LeaderboardEntry } from "../components/LeaderboardCard";
import { TeamLogo } from "../components/BracketGame";
import { useEntries } from "../hooks/useEntries";
import { useForecasts } from "../hooks/useForecasts";
import { useGroupMembers } from "../hooks/useGroupMembers";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import {
  displayName,
  getAllTeamsInBracketOrder,
  truncateAddress,
} from "../lib/tournament";

const teamNames = getAllTeamsInBracketOrder().map((t) => displayName(t));

const PAGE_SIZE = 25;

type SortKey = "score" | "expectedScore" | "winProbability";
type SortDir = "asc" | "desc";

function getChampionName(hex: `0x${string}`): string {
  try {
    return decodeBracket(hex, teamNames).champion;
  } catch {
    return "?";
  }
}

/** Windowed page numbers: always show first/last, window around current. */
function getPageNumbers(
  current: number,
  total: number
): (number | "...")[] {
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
  const pages: (number | "...")[] = [1];
  const start = Math.max(2, current - 1);
  const end = Math.min(total - 1, current + 1);
  if (start > 2) pages.push("...");
  for (let i = start; i <= end; i++) pages.push(i);
  if (end < total - 1) pages.push("...");
  pages.push(total);
  return pages;
}

function SortIndicator({
  active,
  dir,
}: {
  active: boolean;
  dir: SortDir;
}) {
  if (!active)
    return (
      <span className="text-text-muted/30 ml-1 text-[10px]">
        &#x25B2;&#x25BC;
      </span>
    );
  return (
    <span className="text-accent ml-1 text-[10px]">
      {dir === "desc" ? "\u25BC" : "\u25B2"}
    </span>
  );
}

function SortHeader({
  label,
  sortKey: key,
  activeSortKey,
  sortDir,
  onToggle,
  className,
}: {
  label: string;
  sortKey: SortKey;
  activeSortKey: SortKey;
  sortDir: SortDir;
  onToggle: (k: SortKey) => void;
  className?: string;
}) {
  return (
    <th
      className={`py-2 px-2 cursor-pointer select-none whitespace-nowrap ${className ?? ""}`}
      onClick={() => onToggle(key)}
    >
      {label}
      <SortIndicator active={activeSortKey === key} dir={sortDir} />
    </th>
  );
}

export function LeaderboardPage() {
  const { slug } = useParams<{ slug?: string }>();
  const navigate = useNavigate();
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

  const [sortKey, setSortKey] = useState<SortKey>("score");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [page, setPage] = useState(1);

  const hasForecasts =
    forecasts !== null && Object.keys(forecasts).length > 0;

  // Build leaderboard entries (unsorted).
  const rows = useMemo((): LeaderboardEntry[] => {
    if (!entries) return [];

    const out: LeaderboardEntry[] = [];
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
        address,
        tag: entry.name,
        bracket: bracketHex,
        score,
        championName: bracketHex ? getChampionName(bracketHex) : null,
        forecast,
        sortLabel: (entry.name ?? address).toLowerCase(),
      });
    }

    return out;
  }, [entries, forecasts, groupMembers, status]);

  // Sort.
  const sorted = useMemo(() => {
    const arr = [...rows];
    arr.sort((a, b) => {
      let cmp = 0;
      switch (sortKey) {
        case "score": {
          const aS = a.score?.current ?? -1;
          const bS = b.score?.current ?? -1;
          cmp = aS - bS;
          if (cmp === 0) {
            cmp =
              (a.score?.maxPossible ?? -1) - (b.score?.maxPossible ?? -1);
          }
          break;
        }
        case "expectedScore":
          cmp =
            (a.forecast?.expectedScore ?? -1) -
            (b.forecast?.expectedScore ?? -1);
          break;
        case "winProbability":
          cmp =
            (a.forecast?.winProbability ?? -1) -
            (b.forecast?.winProbability ?? -1);
          break;
      }
      if (cmp === 0) cmp = a.sortLabel.localeCompare(b.sortLabel);
      return sortDir === "desc" ? -cmp : cmp;
    });
    return arr;
  }, [rows, sortKey, sortDir]);

  // Pagination.
  const totalPages = Math.max(1, Math.ceil(sorted.length / PAGE_SIZE));
  const safePage = Math.min(page, totalPages);
  const pageRows = sorted.slice(
    (safePage - 1) * PAGE_SIZE,
    safePage * PAGE_SIZE
  );
  const startRank = (safePage - 1) * PAGE_SIZE + 1;

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir((d) => (d === "desc" ? "asc" : "desc"));
    } else {
      setSortKey(key);
      setSortDir("desc");
    }
    setPage(1);
  }

  // --- Error / loading states ---

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

  if (rows.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted">No entries found.</div>
    );
  }

  const decidedCount =
    status?.games.filter((g) => g.status === "final").length ?? 0;
  const liveCount =
    status?.games.filter((g) => g.status === "live").length ?? 0;

  const pageNumbers = getPageNumbers(safePage, totalPages);

  return (
    <div className="w-full mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 mx-2 md:mx-auto md:w-3/4">
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
        <div className="flex gap-3 text-xs text-text-muted">
          {status && (
            <>
              <span>{decidedCount}/63 games decided</span>
              {liveCount > 0 && (
                <span className="text-green-400">{liveCount} live</span>
              )}
            </>
          )}
          <span>{sorted.length} entries</span>
        </div>
      </div>

      {/* Table */}
      <div className="sm:w-3/4 sm:mx-auto overflow-x-auto mx-2">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-text-muted text-xs border-b border-border">
              <th className="text-left py-2 px-2 w-12">#</th>
              <th className="text-left py-2 px-2">Player</th>
              <th className="text-left py-2 px-2 hidden md:table-cell w-44">
                Champion
              </th>
              {hasForecasts && (
                <>
                  <SortHeader
                    label="P(Win)"
                    sortKey="winProbability"
                    activeSortKey={sortKey}
                    sortDir={sortDir}
                    onToggle={toggleSort}
                    className="text-right hidden md:table-cell w-24"
                  />
                  <SortHeader
                    label="E[Score]"
                    sortKey="expectedScore"
                    activeSortKey={sortKey}
                    sortDir={sortDir}
                    onToggle={toggleSort}
                    className="text-right hidden md:table-cell w-24"
                  />
                </>
              )}
              <SortHeader
                label="Score"
                sortKey="score"
                activeSortKey={sortKey}
                sortDir={sortDir}
                onToggle={toggleSort}
                className="text-right"
              />
            </tr>
          </thead>
          <tbody>
            {pageRows.map((entry, i) => {
              const rank = startRank + i;
              return (
                <tr
                  key={entry.address}
                  className={`border-b border-border/20 transition-colors ${
                    entry.bracket
                      ? "cursor-pointer hover:bg-bg-hover/30"
                      : ""
                  }`}
                  onClick={
                    entry.bracket
                      ? () => navigate(`/bracket/${entry.address}`)
                      : undefined
                  }
                >
                  <td className="py-2.5 px-2 text-text-muted font-mono text-xs">
                    {rank}
                  </td>
                  <td className="py-2.5 px-2">
                    <div className="text-text-primary font-mono text-sm truncate max-w-[140px] sm:max-w-none">
                      {entry.tag || truncateAddress(entry.address)}
                    </div>
                    {entry.tag && (
                      <div className="text-[10px] text-text-muted font-mono truncate">
                        {truncateAddress(entry.address)}
                      </div>
                    )}
                  </td>
                  <td className="py-2.5 px-2 hidden md:table-cell">
                    {entry.championName ? (
                      <div className="flex items-center gap-1.5 min-w-0">
                        <TeamLogo teamName={entry.championName} />
                        <span className="text-text-secondary text-xs truncate">
                          {entry.championName}
                        </span>
                      </div>
                    ) : (
                      <span className="text-text-muted">—</span>
                    )}
                  </td>
                  {hasForecasts && (
                    <>
                      <td className="py-2.5 px-2 text-right text-text-muted text-xs hidden md:table-cell">
                        {entry.forecast
                          ? `${(entry.forecast.winProbability * 100).toFixed(1)}%`
                          : "—"}
                      </td>
                      <td className="py-2.5 px-2 text-right text-text-muted text-xs hidden md:table-cell">
                        {entry.forecast
                          ? entry.forecast.expectedScore.toFixed(1)
                          : "—"}
                      </td>
                    </>
                  )}
                  <td className="py-2.5 px-2 text-right whitespace-nowrap">
                    {entry.score ? (
                      <>
                        <span className="text-text-primary font-bold">
                          {entry.score.current}
                        </span>
                        <span className="text-text-muted text-xs">
                          /{entry.score.maxPossible}
                        </span>
                      </>
                    ) : (
                      <span className="text-text-muted">—</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex justify-center items-center gap-1 mt-6 mb-4">
          <button
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={safePage === 1}
            className="px-2 py-1 text-sm text-text-muted hover:text-text-primary disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            &laquo;
          </button>
          {pageNumbers.map((p, i) =>
            p === "..." ? (
              <span
                key={`ellipsis-${i}`}
                className="px-1 text-sm text-text-muted"
              >
                ...
              </span>
            ) : (
              <button
                key={p}
                onClick={() => setPage(p)}
                className={`min-w-[32px] px-2 py-1 text-sm rounded transition-colors ${
                  p === safePage
                    ? "bg-accent text-text-primary font-bold"
                    : "text-text-muted hover:text-text-primary hover:bg-bg-hover/50"
                }`}
              >
                {p}
              </button>
            )
          )}
          <button
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={safePage === totalPages}
            className="px-2 py-1 text-sm text-text-muted hover:text-text-primary disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            &raquo;
          </button>
        </div>
      )}
    </div>
  );
}
