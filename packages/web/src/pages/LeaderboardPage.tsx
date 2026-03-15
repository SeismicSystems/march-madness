import { useMemo } from "react";
import { Link } from "react-router-dom";

import {
  decodeBracket,
  scoreBracketPartial,
  validateBracket,
} from "@march-madness/client";
import type { PartialScore } from "@march-madness/client";

import { useEntries } from "../hooks/useEntries";
import { useTournamentStatus } from "../hooks/useTournamentStatus";
import { getAllTeamsInBracketOrder, truncateAddress } from "../lib/tournament";

interface ScoredEntry {
  address: string;
  tag?: string;
  bracket: `0x${string}`;
  score: PartialScore;
  championName: string;
}

const teamNames = getAllTeamsInBracketOrder().map((t) => t.name);

function getChampionName(hex: `0x${string}`): string {
  try {
    return decodeBracket(hex, teamNames).champion;
  } catch {
    return "?";
  }
}

export function LeaderboardPage() {
  const { entries, loading: entriesLoading } = useEntries();
  const { status, loading: statusLoading } = useTournamentStatus();

  const leaderboard = useMemo((): ScoredEntry[] => {
    if (!entries || !status) return [];

    const scored: ScoredEntry[] = [];
    for (const [address, entry] of Object.entries(entries)) {
      if (!entry.bracket || !validateBracket(entry.bracket)) continue;
      const bracketHex = entry.bracket as `0x${string}`;
      const score = scoreBracketPartial(bracketHex, status);
      scored.push({
        address,
        tag: entry.name,
        bracket: bracketHex,
        score,
        championName: getChampionName(bracketHex),
      });
    }

    scored.sort((a, b) => {
      if (b.score.current !== a.score.current) return b.score.current - a.score.current;
      return b.score.maxPossible - a.score.maxPossible;
    });

    return scored;
  }, [entries, status]);

  const loading = entriesLoading || statusLoading;

  if (loading) {
    return (
      <div className="text-center py-12 text-text-muted">
        Loading leaderboard...
      </div>
    );
  }

  if (!status) {
    return (
      <div className="text-center py-12 text-text-muted">
        Tournament status not available yet.
      </div>
    );
  }

  if (leaderboard.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted">
        No entries found.
      </div>
    );
  }

  const decidedCount = status.games.filter((g) => g.status === "final").length;
  const liveCount = status.games.filter((g) => g.status === "live").length;

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-bold text-text-primary">Leaderboard</h2>
        <div className="flex gap-3 text-xs text-text-muted">
          <span>{decidedCount}/63 games decided</span>
          {liveCount > 0 && (
            <span className="text-green-400">{liveCount} live</span>
          )}
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border text-left text-text-muted">
              <th className="py-2 pr-2 w-10">#</th>
              <th className="py-2 px-2">Player</th>
              <th className="py-2 px-2 text-right">Score</th>
              <th className="py-2 px-2 text-right hidden sm:table-cell">Max</th>
              <th className="py-2 px-2 hidden md:table-cell">Champion</th>
              <th className="py-2 pl-2 w-16"></th>
            </tr>
          </thead>
          <tbody>
            {leaderboard.map((entry, i) => (
              <LeaderboardRow key={entry.address} entry={entry} rank={i + 1} />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function LeaderboardRow({ entry, rank }: { entry: ScoredEntry; rank: number }) {
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
        <span className="text-text-primary font-bold">{entry.score.current}</span>
        <span className="text-text-muted sm:hidden">/{entry.score.maxPossible}</span>
      </td>
      <td className="py-2.5 px-2 text-right text-text-muted hidden sm:table-cell">
        {entry.score.maxPossible}
      </td>
      <td className="py-2.5 px-2 text-text-secondary hidden md:table-cell">
        {entry.championName}
      </td>
      <td className="py-2.5 pl-2">
        <Link
          to={`/bracket/${entry.address}`}
          className="text-xs text-accent hover:text-accent-hover transition-colors"
        >
          View
        </Link>
      </td>
    </tr>
  );
}
