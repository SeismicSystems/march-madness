import { useState, useMemo, useCallback, useEffect } from "react";
import { Link } from "react-router-dom";

import type { ForecastIndex, PartialScore, TournamentStatus } from "@march-madness/client";
import { decodeBracket, scoreBracketPartial } from "@march-madness/client";

import type { TeamProbs } from "../hooks/useTeamProbs";
import { useIsMobile } from "../hooks/useIsMobile";
import { TeamLogo } from "./BracketGame";
import {
  displayAbbrev,
  displayName,
  getAllTeamsInBracketOrder,
  type Team,
} from "../lib/tournament";

/* ── Public interface (reusable for mirrors, groups, leaderboard) ── */

export interface FinalFourEntry {
  id: string;
  label: string;
  bracket: `0x${string}` | null;
}

export interface FinalFourComparisonProps {
  entries: FinalFourEntry[];
  title: string;
  backLink?: { to: string; label: string };
  tournamentStatus: TournamentStatus | null;
  teamProbs: TeamProbs | null;
  onEntryClick?: (entry: FinalFourEntry) => void;
  /** localStorage key suffix for persisting custom entry order (use mirror slug) */
  orderKey?: string;
  /** Per-entry forecasts keyed by entry id (slug). */
  forecasts?: ForecastIndex | null;
}

/* ── Internal types ───────────────────────────────────── */

interface DecodedPicks {
  entry: FinalFourEntry;
  /** 4 F4 teams in region order (East, West, South, Midwest) */
  f4: [Team, Team, Team, Team];
  /** Semifinal winners: [SF1 winner (E/W), SF2 winner (S/MW)] */
  sfWinners: [Team, Team];
  champion: Team;
}

type TeamOverlay = "eliminated" | "advancing" | null;

/* ── Statics ──────────────────────────────────────────── */

const allTeams = getAllTeamsInBracketOrder();
const teamNameList = allTeams.map((t) => displayName(t));
const teamByName = new Map(allTeams.map((t) => [displayName(t), t]));
const STORAGE_PREFIX = "ff-order-";

/* ── Helpers ──────────────────────────────────────────── */

function extractPicks(entry: FinalFourEntry): DecodedPicks | null {
  if (!entry.bracket) return null;
  try {
    const d = decodeBracket(entry.bracket, teamNameList);
    const f4 = d.eliteEight.map((n) => teamByName.get(n));
    const sf = d.finalFour.map((n) => teamByName.get(n));
    const champ = teamByName.get(d.champion);
    if (f4.some((t) => !t) || sf.some((t) => !t) || !champ) return null;
    return {
      entry,
      f4: f4 as [Team, Team, Team, Team],
      sfWinners: sf as [Team, Team],
      champion: champ,
    };
  } catch {
    return null;
  }
}

/**
 * Compute eliminated teams and win counts from the actual tournament bracket
 * (not from any user's picks). Traces the real bracket progression from R64
 * through Championship using tournament status.
 */
function buildTournamentState(status: TournamentStatus) {
  const eliminated = new Set<string>();
  const winCounts = new Map<string, number>();
  let slots: (Team | null)[] = [...allTeams];
  let gi = 0;
  for (let r = 0; r < 6; r++) {
    const n = 32 >> r;
    const next: (Team | null)[] = [];
    for (let g = 0; g < n; g++) {
      const t1 = slots[g * 2];
      const t2 = slots[g * 2 + 1];
      const gs = status.games[gi];
      if (gs?.status === "final" && gs.winner !== undefined && t1 && t2) {
        const w = gs.winner ? t1 : t2;
        const l = gs.winner ? t2 : t1;
        eliminated.add(displayName(l));
        const wn = displayName(w);
        winCounts.set(wn, (winCounts.get(wn) ?? 0) + 1);
        next.push(w);
      } else {
        next.push(null);
      }
      gi++;
    }
    slots = next;
  }
  return { eliminatedTeams: eliminated, winCounts };
}

/**
 * Determine overlay for a team in a specific bracket slot.
 * @param winsNeeded - 4 for F4 slot, 5 for SF winner slot, 6 for champion slot
 */
function getOverlay(
  name: string,
  winsNeeded: number,
  eliminated: Set<string>,
  winCounts: Map<string, number>,
): TeamOverlay {
  if ((winCounts.get(name) ?? 0) >= winsNeeded) return "advancing";
  if (eliminated.has(name)) return "eliminated";
  return null;
}

function reconcileOrder(
  orderKey: string | undefined,
  entryIds: string[],
): string[] {
  if (!orderKey || entryIds.length === 0) return entryIds;
  try {
    const stored = localStorage.getItem(`${STORAGE_PREFIX}${orderKey}`);
    if (!stored) return entryIds;
    const parsed = JSON.parse(stored) as string[];
    const idSet = new Set(entryIds);
    const ordered = parsed.filter((id) => idSet.has(id));
    const seen = new Set(ordered);
    const remaining = entryIds.filter((id) => !seen.has(id));
    return [...ordered, ...remaining];
  } catch {
    return entryIds;
  }
}

/* ── Ordering hook ────────────────────────────────────── */

function useEntryOrder(orderKey: string | undefined, entryIds: string[]) {
  const [order, setOrder] = useState<string[]>(() =>
    reconcileOrder(orderKey, entryIds),
  );

  useEffect(() => {
    setOrder(reconcileOrder(orderKey, entryIds));
  }, [orderKey, entryIds]);

  const moveUp = useCallback(
    (idx: number) => {
      if (idx <= 0) return;
      setOrder((prev) => {
        const next = [...prev];
        [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
        if (orderKey)
          localStorage.setItem(
            `${STORAGE_PREFIX}${orderKey}`,
            JSON.stringify(next),
          );
        return next;
      });
    },
    [orderKey],
  );

  const moveDown = useCallback(
    (idx: number) => {
      setOrder((prev) => {
        if (idx >= prev.length - 1) return prev;
        const next = [...prev];
        [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
        if (orderKey)
          localStorage.setItem(
            `${STORAGE_PREFIX}${orderKey}`,
            JSON.stringify(next),
          );
        return next;
      });
    },
    [orderKey],
  );

  return { order, moveUp, moveDown };
}

/* ── Reorder buttons ─────────────────────────────────── */

function ReorderButtons({
  index,
  total,
  onMoveUp,
  onMoveDown,
}: {
  index: number;
  total: number;
  onMoveUp: (idx: number) => void;
  onMoveDown: (idx: number) => void;
}) {
  return (
    <div className="flex flex-col items-center">
      <button
        onClick={(e) => {
          e.stopPropagation();
          onMoveUp(index);
        }}
        disabled={index === 0}
        className="text-[10px] leading-none text-text-muted/50 hover:text-text-primary disabled:opacity-20 disabled:cursor-default px-1 py-0.5"
      >
        ▲
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onMoveDown(index);
        }}
        disabled={index === total - 1}
        className="text-[10px] leading-none text-text-muted/50 hover:text-text-primary disabled:opacity-20 disabled:cursor-default px-1 py-0.5"
      >
        ▼
      </button>
    </div>
  );
}

/* ── TeamChip ─────────────────────────────────────────── */

function TeamChip({
  team,
  prob,
  ov,
  isChampion,
  compact,
}: {
  team: Team;
  prob?: number;
  ov: TeamOverlay;
  isChampion?: boolean;
  compact?: boolean;
}) {
  const name = displayName(team);

  let cls = "flex items-center gap-1 rounded border ";
  cls += compact ? "px-1 py-px " : "px-1.5 py-0.5 ";

  if (ov === "eliminated") {
    cls += "bg-red-500/10 border-red-500/25 text-red-400/80";
  } else if (ov === "advancing" && isChampion) {
    cls += "bg-gold/15 border-gold/40 text-gold font-bold";
  } else if (ov === "advancing") {
    cls += "bg-green-500/10 border-green-500/40 text-text-primary font-semibold";
  } else {
    cls += "bg-bg-tertiary/40 border-border/30 text-text-primary";
  }

  return (
    <div className={cls}>
      <TeamLogo teamName={name} mobile />
      <span className="text-text-muted text-[10px] flex-shrink-0">
        {team.seed}
      </span>
      <span
        className={`text-xs truncate ${ov === "eliminated" ? "line-through" : ""}`}
      >
        {displayAbbrev(team)}
      </span>
      {prob !== undefined && (
        <span className="text-[9px] text-text-muted/60 flex-shrink-0 ml-auto">
          {Math.round(prob * 100)}%
        </span>
      )}
    </div>
  );
}

/* ── Desktop bracket-flow rows ────────────────────────── */

function DesktopView({
  rows,
  eliminatedTeams,
  winCounts,
  teamProbs,
  onEntryClick,
  onMoveUp,
  onMoveDown,
  forecasts,
  scores,
}: {
  rows: DecodedPicks[];
  eliminatedTeams: Set<string>;
  winCounts: Map<string, number>;
  teamProbs: TeamProbs | null;
  onEntryClick?: (entry: FinalFourEntry) => void;
  onMoveUp: (idx: number) => void;
  onMoveDown: (idx: number) => void;
  forecasts?: ForecastIndex | null;
  scores: Map<string, PartialScore>;
}) {
  const prob = (name: string, idx: number) => teamProbs?.[name]?.[idx];
  const ov = (name: string, wins: number) =>
    getOverlay(name, wins, eliminatedTeams, winCounts);

  return (
    <div className="lg:w-5/6 lg:mx-auto mx-2 space-y-1">
      {rows.map((row, i) => {
        const f4n = row.f4.map((t) => displayName(t));
        const sfn = row.sfWinners.map((t) => displayName(t));
        const cn = displayName(row.champion);
        const fc = forecasts?.[row.entry.id];
        const sc = scores.get(row.entry.id);

        return (
          <div
            key={row.entry.id}
            className={`flex items-center gap-3 px-3 py-1.5 rounded-lg border border-border/20 transition-colors ${
              onEntryClick
                ? "cursor-pointer hover:bg-bg-hover/20 hover:border-border/40"
                : ""
            }`}
            onClick={
              onEntryClick ? () => onEntryClick(row.entry) : undefined
            }
          >
            {/* Entry name */}
            <div className="w-28 shrink-0 font-mono text-sm text-text-primary truncate">
              {row.entry.label}
            </div>

            <div className="shrink-0 flex items-center gap-2 text-[11px] text-text-muted">
              {fc && (
                <>
                  <span>{(fc.winProbability * 100).toFixed(1)}%</span>
                  <span>{fc.expectedScore.toFixed(1)}</span>
                </>
              )}
              {sc && (
                <span className="font-mono">
                  <span className="text-text-primary font-semibold">{sc.current}</span>
                  <span>/{sc.maxPossible}</span>
                </span>
              )}
            </div>

            {/* F4 teams: 2×2 grid */}
            <div className="grid grid-cols-2 gap-x-1.5 gap-y-0.5 shrink-0">
              <TeamChip
                team={row.f4[0]}
                prob={prob(f4n[0], 3)}
                ov={ov(f4n[0], 4)}
                compact
              />
              <TeamChip
                team={row.f4[1]}
                prob={prob(f4n[1], 3)}
                ov={ov(f4n[1], 4)}
                compact
              />
              <TeamChip
                team={row.f4[2]}
                prob={prob(f4n[2], 3)}
                ov={ov(f4n[2], 4)}
                compact
              />
              <TeamChip
                team={row.f4[3]}
                prob={prob(f4n[3], 3)}
                ov={ov(f4n[3], 4)}
                compact
              />
            </div>

            {/* Finalists */}
            <div className="flex flex-col gap-0.5 shrink-0">
              <TeamChip
                team={row.sfWinners[0]}
                prob={prob(sfn[0], 4)}
                ov={ov(sfn[0], 5)}
                compact
              />
              <TeamChip
                team={row.sfWinners[1]}
                prob={prob(sfn[1], 4)}
                ov={ov(sfn[1], 5)}
                compact
              />
            </div>

            {/* Champion */}
            <div className="shrink-0 ml-auto">
              <TeamChip
                team={row.champion}
                prob={prob(cn, 5)}
                ov={ov(cn, 6)}
                isChampion
              />
            </div>

            {/* Reorder */}
            <ReorderButtons
              index={i}
              total={rows.length}
              onMoveUp={onMoveUp}
              onMoveDown={onMoveDown}
            />
          </div>
        );
      })}
    </div>
  );
}

/* ── Mobile cards ─────────────────────────────────────── */

function MobileCards({
  rows,
  eliminatedTeams,
  winCounts,
  teamProbs,
  onEntryClick,
  onMoveUp,
  onMoveDown,
  forecasts,
  scores,
}: {
  rows: DecodedPicks[];
  eliminatedTeams: Set<string>;
  winCounts: Map<string, number>;
  teamProbs: TeamProbs | null;
  onEntryClick?: (entry: FinalFourEntry) => void;
  onMoveUp: (idx: number) => void;
  onMoveDown: (idx: number) => void;
  forecasts?: ForecastIndex | null;
  scores: Map<string, PartialScore>;
}) {
  const prob = (name: string, idx: number) => teamProbs?.[name]?.[idx];
  const ov = (name: string, wins: number) =>
    getOverlay(name, wins, eliminatedTeams, winCounts);

  return (
    <div className="space-y-3 mx-2">
      {rows.map((row, i) => {
        const fc = forecasts?.[row.entry.id];
        const sc = scores.get(row.entry.id);
        const f4n = row.f4.map((t) => displayName(t));
        const sfn = row.sfWinners.map((t) => displayName(t));
        const cn = displayName(row.champion);

        return (
          <div
            key={row.entry.id}
            className={`relative rounded-lg border border-border bg-bg-secondary/50 p-3 ${
              onEntryClick ? "cursor-pointer active:bg-bg-hover/30" : ""
            }`}
            onClick={
              onEntryClick ? () => onEntryClick(row.entry) : undefined
            }
          >
            {/* Move up — top right */}
            <button
              onClick={(e) => {
                e.stopPropagation();
                onMoveUp(i);
              }}
              disabled={i === 0}
              className="absolute top-1.5 right-1.5 text-sm leading-none text-text-muted/50 hover:text-text-primary disabled:opacity-20 disabled:cursor-default px-1.5 py-1"
            >
              ▲
            </button>

            {/* Entry name + stats: win%, expected score, current/max */}
            <div className="flex items-center gap-1.5 mb-2 pr-6">
              <div className="text-sm font-mono font-bold text-text-primary truncate">
                {row.entry.label}
              </div>
              <div className="flex items-center gap-1.5 ml-auto shrink-0 text-[11px] text-text-muted">
                {fc && (
                  <>
                    <span>{(fc.winProbability * 100).toFixed(1)}%</span>
                    <span>{fc.expectedScore.toFixed(1)}</span>
                  </>
                )}
                {sc && (
                  <span className="font-mono">
                    <span className="text-text-primary font-semibold">{sc.current}</span>
                    <span>/{sc.maxPossible}</span>
                  </span>
                )}
              </div>
            </div>

            {/* F4 teams: 2×2 grid — pairs nearly touching */}
            <div className="grid grid-cols-2 gap-x-2 gap-y-px">
              <TeamChip
                team={row.f4[0]}
                prob={prob(f4n[0], 3)}
                ov={ov(f4n[0], 4)}
                compact
              />
              <TeamChip
                team={row.f4[1]}
                prob={prob(f4n[1], 3)}
                ov={ov(f4n[1], 4)}
                compact
              />
              <TeamChip
                team={row.f4[2]}
                prob={prob(f4n[2], 3)}
                ov={ov(f4n[2], 4)}
                compact
              />
              <TeamChip
                team={row.f4[3]}
                prob={prob(f4n[3], 3)}
                ov={ov(f4n[3], 4)}
                compact
              />
            </div>

            {/* Finalists */}
            <div className="grid grid-cols-2 gap-x-2 mt-2.5 pt-2 border-t border-border/15">
              <TeamChip
                team={row.sfWinners[0]}
                prob={prob(sfn[0], 4)}
                ov={ov(sfn[0], 5)}
                compact
              />
              <TeamChip
                team={row.sfWinners[1]}
                prob={prob(sfn[1], 4)}
                ov={ov(sfn[1], 5)}
                compact
              />
            </div>

            {/* Champion + move down */}
            <div className="flex items-center pt-2 mt-2 border-t border-border/30 pr-6">
              <span className="text-[10px] text-gold uppercase tracking-wide shrink-0 mr-1">
                Champion
              </span>
              <div className="flex-1 flex justify-center">
                <TeamChip
                  team={row.champion}
                  prob={prob(cn, 5)}
                  ov={ov(cn, 6)}
                  isChampion
                  compact
                />
              </div>
            </div>
            {/* Move down — bottom right */}
            <button
              onClick={(e) => {
                e.stopPropagation();
                onMoveDown(i);
              }}
              disabled={i === rows.length - 1}
              className="absolute bottom-1.5 right-1.5 text-sm leading-none text-text-muted/50 hover:text-text-primary disabled:opacity-20 disabled:cursor-default px-1.5 py-1"
            >
              ▼
            </button>
          </div>
        );
      })}
    </div>
  );
}

/* ── Main component ───────────────────────────────────── */

export function FinalFourComparison({
  entries,
  title,
  backLink,
  tournamentStatus,
  teamProbs,
  onEntryClick,
  orderKey,
  forecasts,
}: FinalFourComparisonProps) {
  const isMobile = useIsMobile();

  const decoded = useMemo(
    () =>
      entries
        .map(extractPicks)
        .filter((d): d is DecodedPicks => d !== null),
    [entries],
  );

  const defaultIds = useMemo(
    () =>
      [...decoded]
        .sort((a, b) =>
          a.entry.label
            .toLowerCase()
            .localeCompare(b.entry.label.toLowerCase()),
        )
        .map((d) => d.entry.id),
    [decoded],
  );

  const { order, moveUp, moveDown } = useEntryOrder(orderKey, defaultIds);

  const orderedDecoded = useMemo(() => {
    const byId = new Map(decoded.map((d) => [d.entry.id, d]));
    return order
      .map((id) => byId.get(id))
      .filter((d): d is DecodedPicks => d !== null);
  }, [decoded, order]);

  const { eliminatedTeams, winCounts } = useMemo(() => {
    if (!tournamentStatus)
      return {
        eliminatedTeams: new Set<string>(),
        winCounts: new Map<string, number>(),
      };
    return buildTournamentState(tournamentStatus);
  }, [tournamentStatus]);

  const scores = useMemo(() => {
    if (!tournamentStatus) return new Map<string, PartialScore>();
    const m = new Map<string, PartialScore>();
    for (const d of decoded) {
      if (d.entry.bracket) {
        m.set(d.entry.id, scoreBracketPartial(d.entry.bracket, tournamentStatus));
      }
    }
    return m;
  }, [decoded, tournamentStatus]);

  if (orderedDecoded.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted">
        No entries with valid brackets.
      </div>
    );
  }

  const sharedProps = {
    rows: orderedDecoded,
    eliminatedTeams,
    winCounts,
    teamProbs,
    onEntryClick,
    onMoveUp: moveUp,
    onMoveDown: moveDown,
    forecasts,
    scores,
  };

  return (
    <div>
      {/* Header */}
      <div className="flex items-center justify-between mb-4 mx-2 md:mx-auto lg:w-5/6">
        <div className="flex items-center gap-2">
          {backLink && (
            <Link
              to={backLink.to}
              className="text-xs text-accent hover:text-accent-hover transition-colors"
            >
              {backLink.label}
            </Link>
          )}
          {backLink && <span className="text-text-muted">/</span>}
          <h2 className="text-lg font-bold text-text-primary">{title}</h2>
        </div>
        <div className="text-xs text-text-muted">
          {orderedDecoded.length} entries
        </div>
      </div>

      {isMobile ? (
        <MobileCards {...sharedProps} />
      ) : (
        <DesktopView {...sharedProps} />
      )}
    </div>
  );
}
