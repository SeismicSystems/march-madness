import { useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import {
  decodeBracket,
  scoreBracketPartial,
  validateBracket,
} from "@march-madness/client";

import type { BracketForecast, PartialScore } from "@march-madness/client";
import { TeamLogo } from "../components/BracketGame";
import { useMirror } from "../hooks/useMirror";
import { useMirrorForecasts } from "../hooks/useMirrorForecasts";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import {
  displayName,
  getAllTeamsInBracketOrder,
} from "../lib/tournament";

const teamNames = getAllTeamsInBracketOrder().map((t) => displayName(t));

const PAGE_SIZE = 25;

type SortKey = "score" | "expectedScore" | "winProbability";
type SortDir = "asc" | "desc";

interface MirrorLeaderboardEntry {
  entrySlug: string;
  bracket: `0x${string}` | null;
  score: PartialScore | null;
  championName: string | null;
  forecast?: BracketForecast;
  sortLabel: string;
}

function getChampionName(hex: `0x${string}`): string {
  try {
    return decodeBracket(hex, teamNames).champion;
  } catch {
    return "?";
  }
}

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
  label: React.ReactNode;
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

export function MirrorLeaderboardPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { mirror, entries: mirrorEntries, loading: mirrorLoading, notFound, error: mirrorError } = useMirror(id);
  const { status, loading: statusLoading } = useTournamentStatus();
  const { forecasts } = useMirrorForecasts(id);

  const [sortKey, setSortKey] = useState<SortKey>("score");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [page, setPage] = useState(1);
  const [sortTrigger, setSortTrigger] = useState(0);
  const prevSortTrigger = useRef(-1);
  const orderRef = useRef<string[]>([]);

  const hasForecasts =
    forecasts !== null && Object.keys(forecasts).length > 0;

  const rows = useMemo((): MirrorLeaderboardEntry[] => {
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
        entrySlug: entry.slug,
        bracket: bracketHex,
        score,
        championName: bracketHex ? getChampionName(bracketHex) : null,
        forecast,
        sortLabel: entry.slug.toLowerCase(),
      };
    });
  }, [mirrorEntries, forecasts, status]);

  // Sort — only reshuffles on explicit user sort actions, not data updates.
  const sorted = useMemo(() => {
    const rowMap = new Map(rows.map((r) => [r.entrySlug, r]));
    const shouldResort = prevSortTrigger.current !== sortTrigger;

    if (shouldResort || orderRef.current.length === 0) {
      prevSortTrigger.current = sortTrigger;
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
                (a.forecast?.winProbability ?? -1) -
                (b.forecast?.winProbability ?? -1);
            }
            if (cmp === 0) {
              cmp =
                (a.forecast?.expectedScore ?? -1) -
                (b.forecast?.expectedScore ?? -1);
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
      orderRef.current = arr.map((r) => r.entrySlug);
      return arr;
    }

    // Stable order: preserve previous positions, update data in place.
    const result: MirrorLeaderboardEntry[] = [];
    for (const id of orderRef.current) {
      const entry = rowMap.get(id);
      if (entry) {
        result.push(entry);
        rowMap.delete(id);
      }
    }
    for (const entry of rowMap.values()) {
      result.push(entry);
      orderRef.current.push(entry.entrySlug);
    }
    return result;
  }, [rows, sortKey, sortDir, sortTrigger]);

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
    setSortTrigger((n) => n + 1);
    setPage(1);
  }

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
          <h2 className="text-lg font-bold text-text-primary">
            {mirror?.display_name ?? `Mirror ${id}`}
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
              <th className="text-left py-2 px-2">Entry</th>
              <th className="py-2 px-1 md:px-2 w-8 md:w-44" />
              {hasForecasts && (
                <>
                  <SortHeader
                    label={<><span className="md:hidden">{"\u2119"}</span><span className="hidden md:inline">P(Win)</span></>}
                    sortKey="winProbability"
                    activeSortKey={sortKey}
                    sortDir={sortDir}
                    onToggle={toggleSort}
                    className="text-right w-10 md:w-24"
                  />
                  <SortHeader
                    label={<><span className="md:hidden">{"\uD835\uDD3C"}</span><span className="hidden md:inline">E[Score]</span></>}
                    sortKey="expectedScore"
                    activeSortKey={sortKey}
                    sortDir={sortDir}
                    onToggle={toggleSort}
                    className="text-right w-10 md:w-24"
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
                  key={entry.entrySlug}
                  className={`border-b border-border/20 transition-colors ${
                    entry.bracket
                      ? "cursor-pointer hover:bg-bg-hover/30"
                      : ""
                  }`}
                  onClick={
                    entry.bracket
                      ? () =>
                          navigate(
                            `/mirrors/id/${id}/bracket/${entry.entrySlug}`
                          )
                      : undefined
                  }
                >
                  <td className="py-2.5 px-2 text-text-muted font-mono text-xs">
                    {rank}
                  </td>
                  <td className="py-2.5 px-2">
                    <div className="text-text-primary font-mono text-sm truncate max-w-[140px] sm:max-w-none">
                      {entry.entrySlug}
                    </div>
                  </td>
                  <td className="py-2.5 px-1 md:px-2">
                    {entry.championName ? (
                      <div className="flex items-center gap-1.5 min-w-0">
                        <TeamLogo teamName={entry.championName} />
                        <span className="text-text-secondary text-xs truncate hidden md:inline">
                          {entry.championName}
                        </span>
                      </div>
                    ) : (
                      <span className="text-text-muted">—</span>
                    )}
                  </td>
                  {hasForecasts && (
                    <>
                      <td className="py-2.5 px-1 md:px-2 text-right text-text-muted text-[10px] md:text-xs">
                        {entry.forecast
                          ? `${(entry.forecast.winProbability * 100).toFixed(1)}%`
                          : "—"}
                      </td>
                      <td className="py-2.5 px-1 md:px-2 text-right text-text-muted text-[10px] md:text-xs">
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
