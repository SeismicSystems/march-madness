import { useMemo, useState } from "react";

import type { TournamentStatus } from "@march-madness/client";

import type { GameSlot } from "../hooks/useBracket";
import type { TeamProbs } from "../hooks/useTeamProbs";
import { useIsMobile } from "../hooks/useIsMobile";
import { ROUND_NAMES } from "../lib/constants";
import { displayName, tournament, type Team } from "../lib/tournament";
import { BracketGame, TeamLogo } from "./BracketGame";
import { BracketRegion } from "./BracketRegion";
import { FinalFour } from "./FinalFour";

/**
 * Per-game team1 win probability map, keyed by gameIndex.
 * Computed from per-team advance probabilities (Bradley-Terry approximation).
 */
export type GameWinProbs = Map<number, number>;

/**
 * Per-game actual teams based on tournament results (not user picks).
 * For later rounds, the user's bracket may show an eliminated team;
 * this map provides the real teams that advanced.
 */
export type ActualTeamMap = Map<number, { team1: Team | null; team2: Team | null }>;

interface BracketViewProps {
  games: GameSlot[];
  getGamesForRound: (round: number) => GameSlot[];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled?: boolean;
  tournamentStatus?: TournamentStatus;
  /** Per-team advance probabilities from the forecaster (optional). */
  teamProbs?: TeamProbs | null;
}

/**
 * Full bracket layout: 4 regions flowing into Final Four.
 *
 * Desktop:
 *   [East R64→E8]  [Final Four / Championship]  [E8←R64 West]
 *   [South R64→E8] [spacer]                      [E8←R64 Midwest]
 *
 * Mobile: Tabbed stacked lanes — no horizontal scrolling.
 */
export function BracketView({
  games,
  getGamesForRound,
  onPick,
  disabled = false,
  tournamentStatus,
  teamProbs,
}: BracketViewProps) {
  const isMobile = useIsMobile();
  const regions = tournament.regions;
  const displayRegionOrder = [0, 2, 1, 3];
  const displayedRegions = displayRegionOrder.map((index) => ({
    name: regions[index],
    rounds: getRegionGames(index),
  }));

  function getRegionGames(regionIndex: number): GameSlot[][] {
    const rounds: GameSlot[][] = [];
    const r64 = getGamesForRound(0);
    rounds.push(r64.slice(regionIndex * 8, regionIndex * 8 + 8));
    const r32 = getGamesForRound(1);
    rounds.push(r32.slice(regionIndex * 4, regionIndex * 4 + 4));
    const s16 = getGamesForRound(2);
    rounds.push(s16.slice(regionIndex * 2, regionIndex * 2 + 2));
    const e8 = getGamesForRound(3);
    rounds.push(e8.slice(regionIndex, regionIndex + 1));
    return rounds;
  }

  const f4Games = getGamesForRound(4);
  const champGame = getGamesForRound(5);

  // Build actual teams (from tournament results), eliminated set, and advancing set.
  // For later rounds, the user's bracket shows their picks — actualTeams provides
  // the real teams based on who actually won each feeder game.
  const { eliminatedTeams, advancedTeams, actualTeams } = useMemo(() => {
    if (!tournamentStatus)
      return {
        eliminatedTeams: new Set<string>(),
        advancedTeams: new Map<string, number>(),
        actualTeams: null as ActualTeamMap | null,
      };

    const roundStart = [0, 32, 48, 56, 60, 62];
    const actualWinners: (Team | null)[] = new Array(63).fill(null);
    const atMap: ActualTeamMap = new Map();
    const eliminated = new Set<string>();
    const winCounts = new Map<string, number>();

    for (const game of games) {
      // Resolve actual teams for this game
      let at1: Team | null;
      let at2: Team | null;
      if (game.round === 0) {
        at1 = game.team1;
        at2 = game.team2;
      } else {
        const pos = game.gameIndex - roundStart[game.round];
        const feederA = roundStart[game.round - 1] + 2 * pos;
        at1 = actualWinners[feederA];
        at2 = actualWinners[feederA + 1];
      }
      atMap.set(game.gameIndex, { team1: at1, team2: at2 });

      // Record actual winner + eliminated/advanced using actual teams
      const gs = tournamentStatus.games[game.gameIndex];
      if (gs?.status === "final" && gs.winner !== undefined) {
        const winner = gs.winner ? at1 : at2;
        const loser = gs.winner ? at2 : at1;
        actualWinners[game.gameIndex] = winner;
        if (loser) eliminated.add(displayName(loser));
        if (winner) {
          const name = displayName(winner);
          winCounts.set(name, (winCounts.get(name) ?? 0) + 1);
        }
      }
    }

    // Advanced = team name → win count (only teams not yet eliminated)
    const advanced = new Map<string, number>();
    for (const [name, count] of winCounts) {
      if (!eliminated.has(name)) advanced.set(name, count);
    }
    return { eliminatedTeams: eliminated, advancedTeams: advanced, actualTeams: atMap };
  }, [games, tournamentStatus]);

  // Compute per-game team1 win probabilities from team advance probs.
  // Uses Bradley-Terry: P(A wins round r) = advProb_A[r] / (advProb_A[r] + advProb_B[r])
  // Uses actual teams (from tournament results) when available, so probabilities
  // are correct even when the user's bracket has wrong earlier-round picks.
  const gameWinProbs: GameWinProbs = useMemo(() => {
    const map = new Map<number, number>();
    if (!teamProbs) return map;
    for (const game of games) {
      const actual = actualTeams?.get(game.gameIndex);
      const t1 = actual?.team1 ?? game.team1;
      const t2 = actual?.team2 ?? game.team2;
      if (!t1 || !t2) continue;
      const name1 = displayName(t1);
      const name2 = displayName(t2);
      const p1 = teamProbs[name1];
      const p2 = teamProbs[name2];
      if (!p1 || !p2) continue;
      const r = game.round;
      if (r >= p1.length || r >= p2.length) continue;
      const a = p1[r];
      const b = p2[r];
      if (a + b > 0) {
        map.set(game.gameIndex, a / (a + b));
      }
    }
    return map;
  }, [games, teamProbs, actualTeams]);

  if (isMobile) {
    return (
      <MobileBracket
        games={games}
        regions={displayedRegions}
        f4Games={f4Games}
        champGame={champGame}
        onPick={onPick}
        disabled={disabled}
        tournamentStatus={tournamentStatus}
        eliminatedTeams={eliminatedTeams}
        advancedTeams={advancedTeams}
        gameWinProbs={gameWinProbs}
        actualTeams={actualTeams}
      />
    );
  }

  return (
    <div className="pb-4 w-full">
      <div className="grid grid-cols-[1fr_auto_1fr] gap-x-4 gap-y-12 items-stretch w-full min-w-0">
        {/* Top half */}
        <BracketRegion
          regionName={displayedRegions[0].name}
          rounds={displayedRegions[0].rounds}
          onPick={onPick}
          disabled={disabled}
          tournamentStatus={tournamentStatus}
          eliminatedTeams={eliminatedTeams}
          advancedTeams={advancedTeams}
          gameWinProbs={gameWinProbs}
          actualTeams={actualTeams}
        />

        <div className="row-span-2 flex items-center justify-center">
          <FinalFour
            semifinal1={f4Games[0] ?? null}
            semifinal2={f4Games[1] ?? null}
            championship={champGame[0] ?? null}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
            eliminatedTeams={eliminatedTeams}
            advancedTeams={advancedTeams}
            gameWinProbs={gameWinProbs}
            actualTeams={actualTeams}
          />
        </div>

        <BracketRegion
          regionName={displayedRegions[1].name}
          rounds={displayedRegions[1].rounds}
          onPick={onPick}
          disabled={disabled}
          reversed
          tournamentStatus={tournamentStatus}
          eliminatedTeams={eliminatedTeams}
          advancedTeams={advancedTeams}
          gameWinProbs={gameWinProbs}
          actualTeams={actualTeams}
        />

        {/* Bottom half */}
        <BracketRegion
          regionName={displayedRegions[2].name}
          rounds={displayedRegions[2].rounds}
          onPick={onPick}
          disabled={disabled}
          tournamentStatus={tournamentStatus}
          eliminatedTeams={eliminatedTeams}
          advancedTeams={advancedTeams}
          gameWinProbs={gameWinProbs}
          actualTeams={actualTeams}
        />
        <BracketRegion
          regionName={displayedRegions[3].name}
          rounds={displayedRegions[3].rounds}
          onPick={onPick}
          disabled={disabled}
          reversed
          tournamentStatus={tournamentStatus}
          eliminatedTeams={eliminatedTeams}
          advancedTeams={advancedTeams}
          gameWinProbs={gameWinProbs}
          actualTeams={actualTeams}
        />
      </div>
    </div>
  );
}

/* ── Mobile tabbed bracket ─────────────────────────────── */

function MobileBracket({
  games,
  regions,
  f4Games,
  champGame,
  onPick,
  disabled,
  tournamentStatus,
  eliminatedTeams,
  advancedTeams,
  gameWinProbs,
  actualTeams,
}: {
  games: GameSlot[];
  regions: Array<{ name: string; rounds: GameSlot[][] }>;
  f4Games: GameSlot[];
  champGame: GameSlot[];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
  eliminatedTeams: Set<string>;
  advancedTeams: Map<string, number>;
  gameWinProbs: GameWinProbs;
  actualTeams: ActualTeamMap | null;
}) {
  const liveGames = useMemo(() => {
    if (!tournamentStatus) return [];
    return games.filter(
      (g) => tournamentStatus.games[g.gameIndex]?.status === "live",
    );
  }, [games, tournamentStatus]);

  const hasLive = liveGames.length > 0;

  // Compute the global active round across ALL regions (rounds 0-3 only).
  // If any game in round N has status "live" or "final", that round (and all
  // below it) have activity. The highest such round is the active round, and
  // all rounds below it are "settled" and should be pushed to the bottom on
  // mobile region views.
  const settledBeforeRound = useMemo(() => {
    if (!tournamentStatus) return 0;
    let maxActiveRound = -1;
    for (const region of regions) {
      for (let r = 0; r < region.rounds.length; r++) {
        for (const game of region.rounds[r]) {
          const gs = tournamentStatus.games[game.gameIndex];
          if (gs?.status === "live" || gs?.status === "final") {
            if (r > maxActiveRound) maxActiveRound = r;
          }
        }
      }
    }
    // All rounds strictly before the active round are settled
    return maxActiveRound > 0 ? maxActiveRound : 0;
  }, [regions, tournamentStatus]);

  // Track selected tab by stable ID (not index) so inserting/removing the Live
  // tab doesn't shift the user's view. `null` means "no explicit choice" — the
  // default is derived: Live tab if available, otherwise first region.
  // This avoids an awkward flash on initial load: the bracket JSON is bundled
  // (instant) but tournament status is async. While status is loading, tabs
  // show regions only. When status arrives with live games, the Live tab
  // appears and becomes the derived default — no explicit setState needed.
  const [selectedTabId, setSelectedTabId] = useState<string | null>(null);

  const tabs = useMemo(
    () => [
      ...(hasLive ? [{ id: "live", label: "Live" }] : []),
      ...regions.map((r) => ({ id: `region:${r.name}`, label: r.name })),
      { id: "final-four", label: "Final Four" },
    ],
    [hasLive, regions],
  );

  // Resolve active tab: explicit selection if still valid, else derived default
  const defaultTabId = hasLive ? "live" : tabs[0]?.id;
  const activeTabId =
    selectedTabId && tabs.some((t) => t.id === selectedTabId)
      ? selectedTabId
      : defaultTabId;

  // Map active tab ID to content
  const isLiveTab = activeTabId === "live";
  const isFinalFourTab = activeTabId === "final-four";
  const regionIndex = regions.findIndex(
    (r) => `region:${r.name}` === activeTabId,
  );
  const isRegionTab = regionIndex >= 0;

  return (
    <div>
      {/* Tab bar */}
      <div className="flex flex-wrap justify-center gap-1 mb-4">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setSelectedTabId(tab.id)}
            className={`shrink-0 px-4 py-1.5 text-xs rounded-lg border transition-colors ${
              activeTabId === tab.id
                ? tab.id === "live"
                  ? "bg-green-600 text-white border-green-600"
                  : "bg-accent text-white border-accent"
                : tab.id === "live"
                  ? "bg-green-500/10 text-green-400 border-green-500/25 hover:bg-green-500/20"
                  : "bg-bg-tertiary text-text-secondary border-border hover:bg-bg-hover"
            }`}
          >
            {tab.id === "live" ? (
              <span className="flex items-center gap-1.5">
                <span className="relative flex h-1.5 w-1.5">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-green-400" />
                </span>
                Live
              </span>
            ) : (
              tab.label
            )}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="pb-2">
        {isLiveTab ? (
          <MobileLiveGames
            liveGames={liveGames}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
            eliminatedTeams={eliminatedTeams}
            advancedTeams={advancedTeams}
            gameWinProbs={gameWinProbs}
            actualTeams={actualTeams}
          />
        ) : isRegionTab ? (
          <MobileRegionLanes
            regionName={regions[regionIndex].name}
            rounds={regions[regionIndex].rounds}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
            eliminatedTeams={eliminatedTeams}
            advancedTeams={advancedTeams}
            gameWinProbs={gameWinProbs}
            settledBeforeRound={settledBeforeRound}
            actualTeams={actualTeams}
          />
        ) : isFinalFourTab ? (
          <MobileFinalFourLanes
            semifinal1={f4Games[0] ?? null}
            semifinal2={f4Games[1] ?? null}
            championship={champGame[0] ?? null}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
            eliminatedTeams={eliminatedTeams}
            advancedTeams={advancedTeams}
            gameWinProbs={gameWinProbs}
            actualTeams={actualTeams}
          />
        ) : null}
      </div>
    </div>
  );
}

function MobileLiveGames({
  liveGames,
  onPick,
  disabled,
  tournamentStatus,
  eliminatedTeams,
  advancedTeams,
  gameWinProbs,
  actualTeams,
}: {
  liveGames: GameSlot[];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
  eliminatedTeams: Set<string>;
  advancedTeams: Map<string, number>;
  gameWinProbs: GameWinProbs;
  actualTeams: ActualTeamMap | null;
}) {
  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-green-400 uppercase tracking-wider px-1">
        Live Games
      </h3>
      <section className="rounded-lg border border-green-500/25 bg-bg-secondary/70 p-2.5">
        <div className="flex items-center justify-between mb-2">
          <div className="text-[10px] text-green-400/80 uppercase tracking-wide">
            In Progress
          </div>
          <div className="text-[10px] text-text-muted">
            {liveGames.length} game{liveGames.length === 1 ? "" : "s"}
          </div>
        </div>
        <div className="grid grid-cols-1 gap-2">
          {liveGames.map((game) => {
            const at = actualTeams?.get(game.gameIndex);
            return (
              <BracketGame
                key={game.gameIndex}
                team1={game.team1}
                team2={game.team2}
                winner={game.winner}
                onPick={(pickTeam1) => onPick(game.gameIndex, pickTeam1)}
                disabled={disabled}
                mobile
                fullWidth
                round={game.round}
                gameStatus={tournamentStatus?.games[game.gameIndex]}
                eliminatedTeams={eliminatedTeams}
                advancedTeams={advancedTeams}
                team1WinProbability={gameWinProbs.get(game.gameIndex)}
                actualTeam1={at?.team1}
                actualTeam2={at?.team2}
              />
            );
          })}
        </div>
      </section>
    </div>
  );
}

function MobileRegionLanes({
  regionName,
  rounds,
  onPick,
  disabled,
  tournamentStatus,
  eliminatedTeams,
  advancedTeams,
  gameWinProbs,
  settledBeforeRound,
  actualTeams,
}: {
  regionName: string;
  rounds: GameSlot[][];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
  eliminatedTeams: Set<string>;
  advancedTeams: Map<string, number>;
  gameWinProbs: GameWinProbs;
  /** Rounds with index < this value are settled and displayed at the bottom. */
  settledBeforeRound: number;
  actualTeams: ActualTeamMap | null;
}) {
  const reverseLaneColumns = (games: GameSlot[]) => {
    if (games.length < 2) return games;
    const out: GameSlot[] = [];
    for (let i = 0; i < games.length; i += 2) {
      if (i + 1 < games.length) out.push(games[i + 1]);
      out.push(games[i]);
    }
    return out;
  };

  // Reorder rounds: active + future rounds first, then settled rounds at the
  // bottom. E.g. if settledBeforeRound=2 and we have rounds [0,1,2,3], display
  // order becomes [2,3,  0,1] with a "Completed" divider before the settled
  // section.
  const activeRoundIndices = useMemo(() => {
    const active: number[] = [];
    const settled: number[] = [];
    for (let i = 0; i < rounds.length; i++) {
      if (i < settledBeforeRound) {
        settled.push(i);
      } else {
        active.push(i);
      }
    }
    return { active, settled };
  }, [rounds.length, settledBeforeRound]);

  const renderRound = (roundIdx: number, isLast: boolean, isSettled: boolean) => {
    const roundGames = rounds[roundIdx];
    const displayGames =
      roundGames.length > 1 ? reverseLaneColumns(roundGames) : roundGames;

    return (
      <div key={roundIdx} className="space-y-2">
        <section
          className={`rounded-lg border p-2.5 ${
            isSettled
              ? "border-border/50 bg-bg-secondary/40 opacity-60"
              : "border-border bg-bg-secondary/70"
          }`}
        >
          <div className="flex items-center justify-between mb-2">
            <div className="text-[10px] text-text-muted uppercase tracking-wide">
              {ROUND_NAMES[roundIdx]}
            </div>
            <div className="text-[10px] text-text-muted">
              {roundGames.length} game{roundGames.length === 1 ? "" : "s"}
            </div>
          </div>
          <div
            className={`grid gap-2 items-end ${
              roundGames.length > 1 ? "grid-cols-2" : "grid-cols-1"
            }`}
          >
            {displayGames.map((game) => {
              const at = actualTeams?.get(game.gameIndex);
              return (
                <BracketGame
                  key={game.gameIndex}
                  team1={game.team1}
                  team2={game.team2}
                  winner={game.winner}
                  onPick={(pickTeam1) => onPick(game.gameIndex, pickTeam1)}
                  disabled={disabled}
                  compact={roundIdx === 0}
                  mobile
                  fullWidth
                  round={roundIdx}
                  gameStatus={tournamentStatus?.games[game.gameIndex]}
                  eliminatedTeams={eliminatedTeams}
                  advancedTeams={advancedTeams}
                  team1WinProbability={gameWinProbs.get(game.gameIndex)}
                  actualTeam1={at?.team1}
                  actualTeam2={at?.team2}
                />
              );
            })}
          </div>
        </section>
        {!isLast && (
          <div className="flex items-center justify-center gap-2 py-0.5">
            <span className="h-px w-6 bg-border" />
            <span className="text-[9px] uppercase tracking-wide text-text-muted">
              Advance
            </span>
            <span className="h-px w-6 bg-border" />
          </div>
        )}
      </div>
    );
  };

  const hasSettled = activeRoundIndices.settled.length > 0;

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-accent uppercase tracking-wider px-1">
        {regionName}
      </h3>
      {/* Active + future rounds */}
      {activeRoundIndices.active.map((roundIdx, i) =>
        renderRound(
          roundIdx,
          i === activeRoundIndices.active.length - 1 && !hasSettled,
          false,
        ),
      )}
      {/* Settled rounds divider + content */}
      {hasSettled && (
        <>
          <div className="flex items-center justify-center gap-2 py-1">
            <span className="h-px flex-1 bg-border/50" />
            <span className="text-[9px] uppercase tracking-wide text-text-muted/60">
              Completed Rounds
            </span>
            <span className="h-px flex-1 bg-border/50" />
          </div>
          {activeRoundIndices.settled.map((roundIdx, i) =>
            renderRound(
              roundIdx,
              i === activeRoundIndices.settled.length - 1,
              true,
            ),
          )}
        </>
      )}
    </div>
  );
}

function MobileFinalFourLanes({
  semifinal1,
  semifinal2,
  championship,
  onPick,
  disabled,
  tournamentStatus,
  eliminatedTeams,
  advancedTeams,
  gameWinProbs,
  actualTeams,
}: {
  semifinal1: GameSlot | null;
  semifinal2: GameSlot | null;
  championship: GameSlot | null;
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
  eliminatedTeams: Set<string>;
  advancedTeams: Map<string, number>;
  gameWinProbs: GameWinProbs;
  actualTeams: ActualTeamMap | null;
}) {
  const semifinalGames = [semifinal1, semifinal2].filter(
    (g): g is GameSlot => g !== null
  );
  const displaySemifinals =
    semifinalGames.length === 2
      ? [semifinalGames[1], semifinalGames[0]]
      : semifinalGames;

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-accent uppercase tracking-wider px-1">
        Final Four
      </h3>

      <section className="rounded-lg border border-border bg-bg-secondary/70 p-2.5">
        <div className="flex items-center justify-between mb-2">
          <div className="text-[10px] text-text-muted uppercase tracking-wide">
            {ROUND_NAMES[4]}
          </div>
          <div className="text-[10px] text-text-muted">2 games</div>
        </div>
        <div className="grid grid-cols-2 gap-2 items-end">
          {displaySemifinals.map((game) => {
            const at = actualTeams?.get(game.gameIndex);
            return (
              <BracketGame
                key={game.gameIndex}
                team1={game.team1}
                team2={game.team2}
                winner={game.winner}
                onPick={(pickTeam1) => onPick(game.gameIndex, pickTeam1)}
                disabled={disabled}
                mobile
                fullWidth
                round={4}
                gameStatus={tournamentStatus?.games[game.gameIndex]}
                eliminatedTeams={eliminatedTeams}
                advancedTeams={advancedTeams}
                team1WinProbability={gameWinProbs.get(game.gameIndex)}
                actualTeam1={at?.team1}
                actualTeam2={at?.team2}
              />
            );
          })}
        </div>
      </section>

      <div className="flex items-center justify-center gap-2 py-0.5">
        <span className="h-px w-6 bg-border" />
        <span className="text-[9px] uppercase tracking-wide text-text-muted">
          Championship
        </span>
        <span className="h-px w-6 bg-border" />
      </div>

      <section className="rounded-lg border border-border bg-bg-secondary/70 p-2.5">
        <div className="text-[10px] text-text-muted uppercase tracking-wide mb-2">
          {ROUND_NAMES[5]}
        </div>
        {championship && (() => {
          const at = actualTeams?.get(championship.gameIndex);
          return (
            <BracketGame
              team1={championship.team1}
              team2={championship.team2}
              winner={championship.winner}
              onPick={(pickTeam1) => onPick(championship.gameIndex, pickTeam1)}
              disabled={disabled}
              mobile
              fullWidth
              round={5}
              gameStatus={tournamentStatus?.games[championship.gameIndex]}
              eliminatedTeams={eliminatedTeams}
              advancedTeams={advancedTeams}
              team1WinProbability={gameWinProbs.get(championship.gameIndex)}
              actualTeam1={at?.team1}
              actualTeam2={at?.team2}
            />
          );
        })()}
      </section>

      {championship?.winner && (
        <div className="px-3 py-2 bg-gold/15 border border-gold/40 rounded-lg text-center">
          <div className="text-[10px] text-gold/80 uppercase">Champion</div>
          <div className="text-sm font-bold text-gold flex items-center justify-center gap-2">
            <TeamLogo teamName={displayName(championship.winner)} mobile />
            {championship.winner.seed} {displayName(championship.winner)}
          </div>
        </div>
      )}
    </div>
  );
}
