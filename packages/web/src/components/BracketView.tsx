import { useState } from "react";

import type { TournamentStatus } from "@march-madness/client";

import type { GameSlot } from "../hooks/useBracket";
import { useIsMobile } from "../hooks/useIsMobile";
import { tournament } from "../lib/tournament";
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
 * Mobile: Tabbed view — one region at a time + Final Four tab.
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
      <div className="flex overflow-x-auto gap-1 mb-4 pb-1 -mx-1 px-1">
        {TABS.map((tab, i) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(i)}
            className={`shrink-0 px-3 py-1.5 text-xs rounded-lg border transition-colors ${
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
      <div className="overflow-x-auto pb-2">
        {activeTab < 4 ? (
          <BracketRegion
            regionName={regions[activeTab]}
            rounds={getRegionGames(activeTab)}
            onPick={onPick}
            disabled={disabled}
            compact
            tournamentStatus={tournamentStatus}
          />
        ) : (
          <FinalFour
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
