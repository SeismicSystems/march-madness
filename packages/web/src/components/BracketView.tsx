import type { GameSlot } from "../hooks/useBracket";
import { tournament } from "../lib/tournament";
import { BracketRegion } from "./BracketRegion";
import { FinalFour } from "./FinalFour";

interface BracketViewProps {
  games: GameSlot[];
  getGamesForRound: (round: number) => GameSlot[];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled?: boolean;
}

/**
 * Full bracket layout: 4 regions flowing into Final Four.
 *
 * Layout (desktop):
 *   [East R64→E8]  [Final Four / Championship]  [E8←R64 West]
 *   [South R64→E8] [Final Four / Championship]  [E8←R64 Midwest]
 *
 * The left regions read left-to-right, the right regions read right-to-left.
 */
export function BracketView({
  games,
  getGamesForRound,
  onPick,
  disabled = false,
}: BracketViewProps) {
  const regions = tournament.regions; // [East, West, South, Midwest]

  // Each region has 8 R64 games, 4 R32, 2 S16, 1 E8
  // Region 0 (East): R64 games 0-7, R32 games 32-35, S16 games 48-49, E8 game 56
  // Region 1 (West): R64 games 8-15, R32 games 36-39, S16 games 50-51, E8 game 57
  // Region 2 (South): R64 games 16-23, R32 games 40-43, S16 games 52-53, E8 game 58
  // Region 3 (Midwest): R64 games 24-31, R32 games 44-47, S16 games 54-55, E8 game 59

  function getRegionGames(regionIndex: number): GameSlot[][] {
    const rounds: GameSlot[][] = [];
    // R64: 8 games per region
    const r64 = getGamesForRound(0);
    rounds.push(r64.slice(regionIndex * 8, regionIndex * 8 + 8));
    // R32: 4 games per region
    const r32 = getGamesForRound(1);
    rounds.push(r32.slice(regionIndex * 4, regionIndex * 4 + 4));
    // S16: 2 games per region
    const s16 = getGamesForRound(2);
    rounds.push(s16.slice(regionIndex * 2, regionIndex * 2 + 2));
    // E8: 1 game per region
    const e8 = getGamesForRound(3);
    rounds.push(e8.slice(regionIndex, regionIndex + 1));
    return rounds;
  }

  // Final Four and Championship
  const f4Games = getGamesForRound(4);
  const champGame = getGamesForRound(5);

  return (
    <div className="overflow-x-auto pb-4">
      <div className="flex flex-col gap-12 min-w-[1400px]">
        {/* Top half: East (left) + Final Four + West (right) */}
        <div className="flex items-start justify-center gap-4">
          <BracketRegion
            regionName={regions[0]}
            rounds={getRegionGames(0)}
            onPick={onPick}
            disabled={disabled}
          />
          <FinalFour
            semifinal1={f4Games[0] ?? null}
            semifinal2={f4Games[1] ?? null}
            championship={champGame[0] ?? null}
            onPick={onPick}
            disabled={disabled}
          />
          <BracketRegion
            regionName={regions[1]}
            rounds={getRegionGames(1)}
            onPick={onPick}
            disabled={disabled}
            reversed
          />
        </div>

        {/* Bottom half: South (left) + spacer + Midwest (right) */}
        <div className="flex items-start justify-center gap-4">
          <BracketRegion
            regionName={regions[2]}
            rounds={getRegionGames(2)}
            onPick={onPick}
            disabled={disabled}
          />
          {/* Spacer for alignment with Final Four column */}
          <div className="min-w-[200px]" />
          <BracketRegion
            regionName={regions[3]}
            rounds={getRegionGames(3)}
            onPick={onPick}
            disabled={disabled}
            reversed
          />
        </div>
      </div>
    </div>
  );
}
