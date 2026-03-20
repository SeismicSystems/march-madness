import { useMemo } from "react";
import { Link } from "react-router-dom";

import type { TournamentStatus } from "@march-madness/client";
import { decodeBracket } from "@march-madness/client";

import type { TeamProbs } from "../hooks/useTeamProbs";
import { useIsMobile } from "../hooks/useIsMobile";
import { TeamLogo } from "./BracketGame";
import {
  displayAbbrev,
  displayName,
  getAllTeamsInBracketOrder,
  tournament,
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
const regionNames = tournament.regions; // [East, West, South, Midwest]

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
  // Team reached or surpassed this stage → green
  if ((winCounts.get(name) ?? 0) >= winsNeeded) return "advancing";
  // Team didn't reach this stage and is eliminated → red
  if (eliminated.has(name)) return "eliminated";
  // Tournament hasn't progressed here yet
  return null;
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
}: {
  rows: DecodedPicks[];
  eliminatedTeams: Set<string>;
  winCounts: Map<string, number>;
  teamProbs: TeamProbs | null;
  onEntryClick?: (entry: FinalFourEntry) => void;
}) {
  const prob = (name: string, idx: number) => teamProbs?.[name]?.[idx];
  const ov = (name: string, wins: number) =>
    getOverlay(name, wins, eliminatedTeams, winCounts);

  return (
    <div className="lg:w-5/6 lg:mx-auto mx-2 space-y-1">
      {/* Section labels */}
      <div className="flex items-center gap-3 px-3 pb-1 border-b border-border/30">
        <div className="w-28 shrink-0" />
        <div className="flex-1 text-[10px] text-text-muted uppercase tracking-wide text-center">
          Semifinal 1
        </div>
        <div className="flex-1 text-[10px] text-text-muted uppercase tracking-wide text-center">
          Semifinal 2
        </div>
        <div className="w-32 shrink-0 text-[10px] text-gold uppercase tracking-wide text-center">
          Champion
        </div>
      </div>

      {rows.map((row) => {
        const f4n = row.f4.map((t) => displayName(t));
        const sfn = row.sfWinners.map((t) => displayName(t));
        const cn = displayName(row.champion);

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

            {/* SF1: East vs West → Winner */}
            <div className="flex-1 flex items-center gap-1.5 min-w-0">
              <TeamChip
                team={row.f4[0]}
                prob={prob(f4n[0], 3)}
                ov={ov(f4n[0], 4)}
              />
              <span className="text-[10px] text-text-muted/40 shrink-0">
                vs
              </span>
              <TeamChip
                team={row.f4[1]}
                prob={prob(f4n[1], 3)}
                ov={ov(f4n[1], 4)}
              />
              <span className="text-[10px] text-text-muted/40 shrink-0">
                →
              </span>
              <TeamChip
                team={row.sfWinners[0]}
                prob={prob(sfn[0], 4)}
                ov={ov(sfn[0], 5)}
              />
            </div>

            {/* SF2: South vs Midwest → Winner */}
            <div className="flex-1 flex items-center gap-1.5 min-w-0">
              <TeamChip
                team={row.f4[2]}
                prob={prob(f4n[2], 3)}
                ov={ov(f4n[2], 4)}
              />
              <span className="text-[10px] text-text-muted/40 shrink-0">
                vs
              </span>
              <TeamChip
                team={row.f4[3]}
                prob={prob(f4n[3], 3)}
                ov={ov(f4n[3], 4)}
              />
              <span className="text-[10px] text-text-muted/40 shrink-0">
                →
              </span>
              <TeamChip
                team={row.sfWinners[1]}
                prob={prob(sfn[1], 4)}
                ov={ov(sfn[1], 5)}
              />
            </div>

            {/* Champion */}
            <div className="w-32 shrink-0 flex justify-center">
              <TeamChip
                team={row.champion}
                prob={prob(cn, 5)}
                ov={ov(cn, 6)}
                isChampion
              />
            </div>
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
}: {
  rows: DecodedPicks[];
  eliminatedTeams: Set<string>;
  winCounts: Map<string, number>;
  teamProbs: TeamProbs | null;
  onEntryClick?: (entry: FinalFourEntry) => void;
}) {
  const prob = (name: string, idx: number) => teamProbs?.[name]?.[idx];
  const ov = (name: string, wins: number) =>
    getOverlay(name, wins, eliminatedTeams, winCounts);

  return (
    <div className="space-y-3 mx-2">
      {rows.map((row) => {
        const f4n = row.f4.map((t) => displayName(t));
        const sfn = row.sfWinners.map((t) => displayName(t));
        const cn = displayName(row.champion);

        return (
          <div
            key={row.entry.id}
            className={`rounded-lg border border-border bg-bg-secondary/50 p-3 ${
              onEntryClick ? "cursor-pointer active:bg-bg-hover/30" : ""
            }`}
            onClick={
              onEntryClick ? () => onEntryClick(row.entry) : undefined
            }
          >
            {/* Entry name */}
            <div className="text-sm font-mono font-bold text-text-primary mb-2.5">
              {row.entry.label}
            </div>

            {/* Two semifinal columns */}
            <div className="grid grid-cols-2 gap-3 mb-2.5">
              {/* SF1: East vs West */}
              <div className="space-y-1.5">
                <div className="text-[10px] text-text-muted uppercase tracking-wide">
                  Semifinal 1
                </div>
                <div className="space-y-1">
                  <TeamChip
                    team={row.f4[0]}
                    prob={prob(f4n[0], 3)}
                    ov={ov(f4n[0], 4)}
                    compact
                  />
                  <div className="text-[9px] text-text-muted/50 text-center">
                    vs
                  </div>
                  <TeamChip
                    team={row.f4[1]}
                    prob={prob(f4n[1], 3)}
                    ov={ov(f4n[1], 4)}
                    compact
                  />
                </div>
                <div className="flex items-center gap-1 pt-0.5">
                  <span className="text-[9px] text-text-muted/50">→</span>
                  <div className="flex-1">
                    <TeamChip
                      team={row.sfWinners[0]}
                      prob={prob(sfn[0], 4)}
                      ov={ov(sfn[0], 5)}
                      compact
                    />
                  </div>
                </div>
              </div>

              {/* SF2: South vs Midwest */}
              <div className="space-y-1.5">
                <div className="text-[10px] text-text-muted uppercase tracking-wide">
                  Semifinal 2
                </div>
                <div className="space-y-1">
                  <TeamChip
                    team={row.f4[2]}
                    prob={prob(f4n[2], 3)}
                    ov={ov(f4n[2], 4)}
                    compact
                  />
                  <div className="text-[9px] text-text-muted/50 text-center">
                    vs
                  </div>
                  <TeamChip
                    team={row.f4[3]}
                    prob={prob(f4n[3], 3)}
                    ov={ov(f4n[3], 4)}
                    compact
                  />
                </div>
                <div className="flex items-center gap-1 pt-0.5">
                  <span className="text-[9px] text-text-muted/50">→</span>
                  <div className="flex-1">
                    <TeamChip
                      team={row.sfWinners[1]}
                      prob={prob(sfn[1], 4)}
                      ov={ov(sfn[1], 5)}
                      compact
                    />
                  </div>
                </div>
              </div>
            </div>

            {/* Champion */}
            <div className="flex items-center justify-center gap-2 pt-2 border-t border-border/30">
              <span className="text-[10px] text-gold uppercase tracking-wide">
                Champion
              </span>
              <TeamChip
                team={row.champion}
                prob={prob(cn, 5)}
                ov={ov(cn, 6)}
                isChampion
                compact
              />
            </div>
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
}: FinalFourComparisonProps) {
  const isMobile = useIsMobile();

  const decoded = useMemo(() => {
    const raw = entries
      .map(extractPicks)
      .filter((d): d is DecodedPicks => d !== null);
    raw.sort((a, b) =>
      a.entry.label.toLowerCase().localeCompare(b.entry.label.toLowerCase()),
    );
    return raw;
  }, [entries]);

  const { eliminatedTeams, winCounts } = useMemo(() => {
    if (!tournamentStatus)
      return {
        eliminatedTeams: new Set<string>(),
        winCounts: new Map<string, number>(),
      };
    return buildTournamentState(tournamentStatus);
  }, [tournamentStatus]);

  if (decoded.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted">
        No entries with valid brackets.
      </div>
    );
  }

  const sharedProps = {
    rows: decoded,
    eliminatedTeams,
    winCounts,
    teamProbs,
    onEntryClick,
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
          {decoded.length} entries
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
