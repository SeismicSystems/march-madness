import { useState } from "react";

import type { TournamentStatus } from "@march-madness/client";

import type { GameSlot } from "../hooks/useBracket";
import { useIsMobile } from "../hooks/useIsMobile";
import { ROUND_NAMES } from "../lib/constants";
import { tournament } from "../lib/tournament";
import { BracketGame, TeamLogo } from "./BracketGame";
import { BracketRegion } from "./BracketRegion";
import { FinalFour } from "./FinalFour";

interface BracketViewProps {
  games: GameSlot[];
  getGamesForRound: (round: number) => GameSlot[];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled?: boolean;
  tournamentStatus?: TournamentStatus;
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
}: BracketViewProps) {
  const isMobile = useIsMobile();
  const regions = tournament.regions; // [East, West, South, Midwest]

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

  if (isMobile) {
    return (
      <MobileBracket
        regions={regions}
        getRegionGames={getRegionGames}
        f4Games={f4Games}
        champGame={champGame}
        onPick={onPick}
        disabled={disabled}
        tournamentStatus={tournamentStatus}
      />
    );
  }

  return (
    <div className="overflow-x-auto pb-4">
      <div className="grid grid-cols-[1fr_auto_1fr] gap-x-4 gap-y-12 min-w-[1400px] items-stretch">
        {/* Top half: East (left) + Final Four + West (right) */}
        <BracketRegion
          regionName={regions[0]}
          rounds={getRegionGames(0)}
          onPick={onPick}
          disabled={disabled}
          tournamentStatus={tournamentStatus}
        />

        <div className="row-span-2 flex items-center justify-center">
          <FinalFour
            semifinal1={f4Games[0] ?? null}
            semifinal2={f4Games[1] ?? null}
            championship={champGame[0] ?? null}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
          />
        </div>

        <BracketRegion
          regionName={regions[1]}
          rounds={getRegionGames(1)}
          onPick={onPick}
          disabled={disabled}
          reversed
          tournamentStatus={tournamentStatus}
        />

        {/* Bottom half: South (left) + Midwest (right) */}
        <BracketRegion
          regionName={regions[2]}
          rounds={getRegionGames(2)}
          onPick={onPick}
          disabled={disabled}
          tournamentStatus={tournamentStatus}
        />
        <BracketRegion
          regionName={regions[3]}
          rounds={getRegionGames(3)}
          onPick={onPick}
          disabled={disabled}
          reversed
          tournamentStatus={tournamentStatus}
        />
      </div>
    </div>
  );
}

/* ── Mobile tabbed bracket ─────────────────────────────── */

const TABS = ["East", "West", "South", "Midwest", "Final Four"] as const;

function MobileBracket({
  regions,
  getRegionGames,
  f4Games,
  champGame,
  onPick,
  disabled,
  tournamentStatus,
}: {
  regions: string[];
  getRegionGames: (i: number) => GameSlot[][];
  f4Games: GameSlot[];
  champGame: GameSlot[];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
}) {
  const [activeTab, setActiveTab] = useState(0);

  return (
    <div>
      {/* Tab bar */}
      <div className="flex flex-wrap justify-center gap-1 mb-4">
        {TABS.map((tab, i) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(i)}
            className={`shrink-0 px-4 py-1.5 text-xs rounded-lg border transition-colors ${
              activeTab === i
                ? "bg-accent text-white border-accent"
                : "bg-bg-tertiary text-text-secondary border-border hover:bg-bg-hover"
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="pb-2">
        {activeTab < 4 ? (
          <MobileRegionLanes
            regionName={regions[activeTab]}
            rounds={getRegionGames(activeTab)}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
          />
        ) : (
          <MobileFinalFourLanes
            semifinal1={f4Games[0] ?? null}
            semifinal2={f4Games[1] ?? null}
            championship={champGame[0] ?? null}
            onPick={onPick}
            disabled={disabled}
            tournamentStatus={tournamentStatus}
          />
        )}
      </div>
    </div>
  );
}

function MobileRegionLanes({
  regionName,
  rounds,
  onPick,
  disabled,
  tournamentStatus,
}: {
  regionName: string;
  rounds: GameSlot[][];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
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

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-accent uppercase tracking-wider px-1">
        {regionName}
      </h3>
      {rounds.map((roundGames, roundIdx) => {
        const displayGames =
          roundGames.length > 1 ? reverseLaneColumns(roundGames) : roundGames;

        return (
          <div key={roundIdx} className="space-y-2">
            <section className="rounded-lg border border-border bg-bg-secondary/70 p-2.5">
              <div className="flex items-center justify-between mb-2">
                <div className="text-[10px] text-text-muted uppercase tracking-wide">
                  {ROUND_NAMES[roundIdx]}
                </div>
                <div className="text-[10px] text-text-muted">
                  {roundGames.length} game{roundGames.length === 1 ? "" : "s"}
                </div>
              </div>
              <div
                className={`grid gap-2 ${roundGames.length > 1 ? "grid-cols-2" : "grid-cols-1"}`}
              >
                {displayGames.map((game) => (
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
                    gameStatus={tournamentStatus?.games[game.gameIndex]}
                  />
                ))}
              </div>
            </section>
            {roundIdx < rounds.length - 1 && (
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
      })}
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
}: {
  semifinal1: GameSlot | null;
  semifinal2: GameSlot | null;
  championship: GameSlot | null;
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled: boolean;
  tournamentStatus?: TournamentStatus;
}) {
  const semifinalGames = [semifinal1, semifinal2].filter(
    (g): g is GameSlot => g !== null,
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
        <div className="grid grid-cols-2 gap-2">
          {displaySemifinals.map((game) => (
            <BracketGame
              key={game.gameIndex}
              team1={game.team1}
              team2={game.team2}
              winner={game.winner}
              onPick={(pickTeam1) => onPick(game.gameIndex, pickTeam1)}
              disabled={disabled}
              mobile
              fullWidth
              gameStatus={tournamentStatus?.games[game.gameIndex]}
            />
          ))}
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
        {championship && (
          <BracketGame
            team1={championship.team1}
            team2={championship.team2}
            winner={championship.winner}
            onPick={(pickTeam1) => onPick(championship.gameIndex, pickTeam1)}
            disabled={disabled}
            mobile
            fullWidth
            gameStatus={tournamentStatus?.games[championship.gameIndex]}
          />
        )}
      </section>

      {championship?.winner && (
        <div className="px-3 py-2 bg-gold/15 border border-gold/40 rounded-lg text-center">
          <div className="text-[10px] text-gold/80 uppercase">Champion</div>
          <div className="text-sm font-bold text-gold flex items-center justify-center gap-2">
            <TeamLogo teamName={championship.winner.name} mobile />
            {championship.winner.seed} {championship.winner.name}
          </div>
        </div>
      )}
    </div>
  );
}
