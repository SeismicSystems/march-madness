import { useState, useMemo, useCallback, useEffect } from "react";
import { Link } from "react-router-dom";

import type {
  ForecastIndex,
  PartialScore,
  TournamentStatus,
} from "@march-madness/client";
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

interface ChampionGroup {
  label: string;
  champTeam?: Team;
  entries: DecodedPicks[];
}

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

/* ── Champion grouping ────────────────────────────────── */

/**
 * Group entries by champion pick for the desktop tiled view.
 *
 * Fully data-driven — no hardcoded caps or minimums.
 *
 * 1. Build team→count and seed→count maps from the pool.
 * 2. Compute a dynamic threshold for "own section" that scales with pool
 *    size: `max(2, ceil(total / 10))`. Bigger pools need a bigger share
 *    to justify a dedicated section.
 * 3. Teams above the threshold get named sections.
 * 4. Remainder is grouped by seed: seeds with ≥ 2 remainder entries get
 *    their own "Other N-seeds" section; tiny seeds merge into one bucket.
 *    If only one seed tier remains, the merged bucket names it; if 2+
 *    tiers, it becomes "Other seeds".
 *
 * Final ordering: named teams (count desc) → single-seed groups (seed asc)
 * → multi-seed merged bucket.
 *
 * Within each group the relative order of `rows` is preserved so the
 * active sort mode carries through.
 */
function deriveChampionGroups(rows: DecodedPicks[]): ChampionGroup[] {
  const total = rows.length;

  // ── Step 1: build team→{team, entries} map (preserving row order) ──
  const byChamp = new Map<
    string,
    { team: Team; entries: DecodedPicks[] }
  >();
  for (const row of rows) {
    const name = displayName(row.champion);
    let bucket = byChamp.get(name);
    if (!bucket) {
      bucket = { team: row.champion, entries: [] };
      byChamp.set(name, bucket);
    }
    bucket.entries.push(row);
  }

  // ── Step 2: dynamic threshold ──
  // Scales with pool size: 14→2, 20→2, 30→3, 50→5, 100→10
  const ownThreshold = Math.max(2, Math.ceil(total / 10));

  // Sort teams by count desc, then alphabetically for ties.
  const sorted = [...byChamp.entries()].sort(
    (a, b) =>
      b[1].entries.length - a[1].entries.length ||
      a[0].localeCompare(b[0]),
  );

  // ── Step 3: named sections ──
  const named: ChampionGroup[] = [];
  const remainder: DecodedPicks[] = [];

  for (const [champName, { team, entries }] of sorted) {
    if (entries.length >= ownThreshold) {
      named.push({ label: champName, champTeam: team, entries });
    } else {
      remainder.push(...entries);
    }
  }

  if (remainder.length === 0) return named;

  // ── Step 4: group remainder by champion seed ──
  const bySeed = new Map<number, DecodedPicks[]>();
  for (const row of remainder) {
    const seed = row.champion.seed;
    let arr = bySeed.get(seed);
    if (!arr) {
      arr = [];
      bySeed.set(seed, arr);
    }
    arr.push(row);
  }

  // Single seed in remainder — one group, name it directly.
  if (bySeed.size === 1) {
    const [seed, entries] = [...bySeed.entries()][0];
    named.push({ label: `Other ${seed}-seeds`, entries });
    return named;
  }

  // Multiple seeds: split into single-seed groups (≥ 2 entries) and a
  // merged bucket for the rest.
  const singleSeedGroups: ChampionGroup[] = [];
  const merged: DecodedPicks[] = [];
  const mergedSeeds: number[] = [];

  for (const [seed, entries] of [...bySeed.entries()].sort(
    (a, b) => a[0] - b[0],
  )) {
    if (entries.length >= 2) {
      singleSeedGroups.push({
        label: `Other ${seed}-seeds`,
        entries,
      });
    } else {
      merged.push(...entries);
      mergedSeeds.push(seed);
    }
  }

  // Name the merged bucket based on how many seed tiers it contains.
  if (merged.length > 0) {
    const label =
      mergedSeeds.length === 1
        ? `Other ${mergedSeeds[0]}-seeds`
        : "Other seeds";
    singleSeedGroups.push({ label, entries: merged });
  }

  // ── Final order: named teams → single-seed groups → merged bucket ──
  return [...named, ...singleSeedGroups];
}

/* ── Sort modes ───────────────────────────────────────── */

type SortMode = "custom" | "prob" | "expected" | "current";

/* ── Ordering hook ────────────────────────────────────── */

function useEntryOrder(orderKey: string | undefined, entryIds: string[]) {
  const [order, setOrder] = useState<string[]>(() =>
    reconcileOrder(orderKey, entryIds),
  );

  useEffect(() => {
    setOrder(reconcileOrder(orderKey, entryIds));
  }, [orderKey, entryIds]);

  const persistOrder = useCallback(
    (ids: string[]) => {
      if (orderKey)
        localStorage.setItem(
          `${STORAGE_PREFIX}${orderKey}`,
          JSON.stringify(ids),
        );
    },
    [orderKey],
  );

  const setCustomOrder = useCallback(
    (ids: string[]) => {
      setOrder(ids);
      persistOrder(ids);
    },
    [persistOrder],
  );

  const moveUp = useCallback(
    (idx: number) => {
      if (idx <= 0) return;
      setOrder((prev) => {
        const next = [...prev];
        [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
        persistOrder(next);
        return next;
      });
    },
    [persistOrder],
  );

  const moveDown = useCallback(
    (idx: number) => {
      setOrder((prev) => {
        if (idx >= prev.length - 1) return prev;
        const next = [...prev];
        [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
        persistOrder(next);
        return next;
      });
    },
    [persistOrder],
  );

  return { order, moveUp, moveDown, setCustomOrder };
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

/* ── Entry card (shared between mobile list + desktop grid) ── */

function EntryCard({
  row,
  prob,
  ov,
  fc,
  sc,
  onEntryClick,
  reorder,
}: {
  row: DecodedPicks;
  prob: (name: string, idx: number) => number | undefined;
  ov: (name: string, wins: number) => TeamOverlay;
  fc?: { winProbability: number; expectedScore: number };
  sc?: PartialScore;
  onEntryClick?: (entry: FinalFourEntry) => void;
  /** If provided, show ▲/▼ buttons and "Bracket →" instead of card click. */
  reorder?: {
    index: number;
    total: number;
    onMoveUp: (i: number) => void;
    onMoveDown: (i: number) => void;
  };
}) {
  const f4n = row.f4.map((t) => displayName(t));
  const sfn = row.sfWinners.map((t) => displayName(t));
  const cn = displayName(row.champion);
  const cardClickable = !reorder && onEntryClick;

  return (
    <div
      className={`relative rounded-lg border border-border bg-bg-secondary/50 p-3 ${
        cardClickable
          ? "cursor-pointer hover:bg-bg-hover/20 hover:border-border/40 transition-colors"
          : ""
      }`}
      onClick={cardClickable ? () => onEntryClick(row.entry) : undefined}
    >
      {/* Move up — top right (mobile only) */}
      {reorder && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            reorder.onMoveUp(reorder.index);
          }}
          disabled={reorder.index === 0}
          className="absolute top-2 right-3 text-sm leading-none text-text-muted/50 hover:text-text-primary disabled:opacity-20 disabled:cursor-default px-1 py-0.5"
        >
          ▲
        </button>
      )}

      {/* Entry name + stats */}
      <div
        className={`flex items-center gap-1.5 mb-2 ${reorder ? "pr-8" : ""}`}
      >
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
            <span className="font-mono ml-1">
              <span className="text-text-primary font-semibold">
                {sc.current}
              </span>
              <span>/{sc.maxPossible}</span>
            </span>
          )}
        </div>
      </div>

      {/* F4 teams: 2×2 grid */}
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

      {/* Bottom row */}
      <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-1 pt-2 mt-2 border-t border-border/30">
        <div className="flex justify-start">
          {reorder && onEntryClick && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onEntryClick(row.entry);
              }}
              className="text-xs text-accent hover:text-accent-hover active:text-accent-hover font-medium px-2 py-1 rounded border border-accent/30 hover:border-accent/60 transition-colors"
            >
              Bracket →
            </button>
          )}
        </div>
        <TeamChip
          team={row.champion}
          prob={prob(cn, 5)}
          ov={ov(cn, 6)}
          isChampion
          compact
        />
        <div className="flex justify-end pr-1">
          {reorder && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                reorder.onMoveDown(reorder.index);
              }}
              disabled={reorder.index === reorder.total - 1}
              className="text-sm leading-none text-text-muted/50 hover:text-text-primary disabled:opacity-20 disabled:cursor-default px-1 py-0.5"
            >
              ▼
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/* ── Desktop grouped view ─────────────────────────────── */

function DesktopGroupedView({
  rows,
  eliminatedTeams,
  winCounts,
  teamProbs,
  onEntryClick,
  forecasts,
  scores,
}: {
  rows: DecodedPicks[];
  eliminatedTeams: Set<string>;
  winCounts: Map<string, number>;
  teamProbs: TeamProbs | null;
  onEntryClick?: (entry: FinalFourEntry) => void;
  forecasts?: ForecastIndex | null;
  scores: Map<string, PartialScore>;
}) {
  const prob = (name: string, idx: number) => teamProbs?.[name]?.[idx];
  const ov = (name: string, wins: number) =>
    getOverlay(name, wins, eliminatedTeams, winCounts);

  const groups = useMemo(() => deriveChampionGroups(rows), [rows]);

  return (
    <div className="lg:w-5/6 lg:mx-auto mx-2 space-y-6">
      {groups.map((group) => (
        <div key={group.label}>
          {/* Section header */}
          <div className="flex items-center gap-2 mb-2 pb-1.5 border-b border-border/30">
            {group.champTeam && (
              <TeamLogo
                teamName={displayName(group.champTeam)}
                mobile
              />
            )}
            <span className="text-sm font-semibold text-text-primary">
              {group.label}
            </span>
            <span className="text-[10px] text-text-muted">
              {group.entries.length}
            </span>
          </div>

          {/* Card grid */}
          <div className="grid grid-cols-2 xl:grid-cols-3 gap-3">
            {group.entries.map((row) => (
              <EntryCard
                key={row.entry.id}
                row={row}
                prob={prob}
                ov={ov}
                fc={forecasts?.[row.entry.id]}
                sc={scores.get(row.entry.id)}
                onEntryClick={onEntryClick}
              />
            ))}
          </div>
        </div>
      ))}
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
      {rows.map((row, i) => (
        <EntryCard
          key={row.entry.id}
          row={row}
          prob={prob}
          ov={ov}
          fc={forecasts?.[row.entry.id]}
          sc={scores.get(row.entry.id)}
          onEntryClick={onEntryClick}
          reorder={{
            index: i,
            total: rows.length,
            onMoveUp,
            onMoveDown,
          }}
        />
      ))}
    </div>
  );
}

/* ── Sort footer ──────────────────────────────────────── */

const SORT_OPTIONS: { mode: SortMode; label: string }[] = [
  { mode: "prob", label: "Win %" },
  { mode: "expected", label: "E[pts]" },
  { mode: "current", label: "Score" },
];

function SortFooter({
  sortMode,
  onToggle,
}: {
  sortMode: SortMode;
  onToggle: (mode: SortMode) => void;
}) {
  return (
    <div className="flex items-center justify-center gap-2 mt-4 mx-2 md:mx-auto lg:w-5/6">
      <span className="text-[10px] text-text-muted uppercase tracking-wide mr-1">
        Sort
      </span>
      {SORT_OPTIONS.map(({ mode, label }) => (
        <button
          key={mode}
          onClick={() => onToggle(mode)}
          className={`text-xs px-2.5 py-1 rounded border transition-colors ${
            sortMode === mode
              ? "border-accent/60 bg-accent/10 text-accent font-medium"
              : "border-border/30 text-text-muted hover:border-border/60 hover:text-text-primary"
          }`}
        >
          {label}
        </button>
      ))}
      {sortMode !== "custom" && (
        <button
          onClick={() => onToggle("custom")}
          className="text-[10px] text-text-muted/60 hover:text-text-primary ml-1"
        >
          ↩ Saved order
        </button>
      )}
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

  const { order, moveUp, moveDown, setCustomOrder } = useEntryOrder(
    orderKey,
    defaultIds,
  );
  const [sortMode, setSortMode] = useState<SortMode>("custom");

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
        m.set(
          d.entry.id,
          scoreBracketPartial(d.entry.bracket, tournamentStatus),
        );
      }
    }
    return m;
  }, [decoded, tournamentStatus]);

  const displayRows = useMemo(() => {
    if (sortMode === "custom") return orderedDecoded;
    const sorted = [...orderedDecoded];
    sorted.sort((a, b) => {
      const fa = forecasts?.[a.entry.id];
      const fb = forecasts?.[b.entry.id];
      const sa = scores.get(a.entry.id);
      const sb = scores.get(b.entry.id);
      switch (sortMode) {
        case "prob":
          return (fb?.winProbability ?? 0) - (fa?.winProbability ?? 0);
        case "expected":
          return (fb?.expectedScore ?? 0) - (fa?.expectedScore ?? 0);
        case "current":
          return (sb?.current ?? 0) - (sa?.current ?? 0);
      }
    });
    return sorted;
  }, [sortMode, orderedDecoded, forecasts, scores]);

  const handleMoveUp = useCallback(
    (idx: number) => {
      if (sortMode !== "custom") {
        const ids = displayRows.map((d) => d.entry.id);
        if (idx <= 0) return;
        [ids[idx - 1], ids[idx]] = [ids[idx], ids[idx - 1]];
        setCustomOrder(ids);
        setSortMode("custom");
      } else {
        moveUp(idx);
      }
    },
    [sortMode, displayRows, setCustomOrder, moveUp],
  );

  const handleMoveDown = useCallback(
    (idx: number) => {
      if (sortMode !== "custom") {
        const ids = displayRows.map((d) => d.entry.id);
        if (idx >= ids.length - 1) return;
        [ids[idx], ids[idx + 1]] = [ids[idx + 1], ids[idx]];
        setCustomOrder(ids);
        setSortMode("custom");
      } else {
        moveDown(idx);
      }
    },
    [sortMode, displayRows, setCustomOrder, moveDown],
  );

  const toggleSort = useCallback((mode: SortMode) => {
    setSortMode((prev) => (prev === mode ? "custom" : mode));
  }, []);

  if (displayRows.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted">
        No entries with valid brackets.
      </div>
    );
  }

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
          {displayRows.length} entries
        </div>
      </div>

      {isMobile ? (
        <MobileCards
          rows={displayRows}
          eliminatedTeams={eliminatedTeams}
          winCounts={winCounts}
          teamProbs={teamProbs}
          onEntryClick={onEntryClick}
          onMoveUp={handleMoveUp}
          onMoveDown={handleMoveDown}
          forecasts={forecasts}
          scores={scores}
        />
      ) : (
        <DesktopGroupedView
          rows={displayRows}
          eliminatedTeams={eliminatedTeams}
          winCounts={winCounts}
          teamProbs={teamProbs}
          onEntryClick={onEntryClick}
          forecasts={forecasts}
          scores={scores}
        />
      )}

      {/* Sort footer */}
      <SortFooter sortMode={sortMode} onToggle={toggleSort} />
    </div>
  );
}
