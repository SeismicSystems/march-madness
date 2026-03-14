import type { GameSlot } from "../hooks/useBracket";
import { ROUND_NAMES } from "../lib/constants";
import { BracketGame } from "./BracketGame";

interface BracketRegionProps {
  regionName: string;
  /** Games for rounds 0-3 in this region (R64 through E8) */
  rounds: GameSlot[][];
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled?: boolean;
  /** Whether this region reads left-to-right or right-to-left */
  reversed?: boolean;
}

export function BracketRegion({
  regionName,
  rounds,
  onPick,
  disabled = false,
  reversed = false,
}: BracketRegionProps) {
  const orderedRounds = reversed ? [...rounds].reverse() : rounds;

  return (
    <div className="flex flex-col">
      <h3 className="text-sm font-semibold text-accent uppercase tracking-wider mb-3 px-1">
        {regionName}
      </h3>
      <div className={`flex ${reversed ? "flex-row-reverse" : "flex-row"} items-center gap-1`}>
        {orderedRounds.map((roundGames, displayIdx) => {
          const actualRoundIdx = reversed
            ? rounds.length - 1 - displayIdx
            : displayIdx;
          const roundSpacing = getVerticalSpacing(actualRoundIdx);

          return (
            <div
              key={displayIdx}
              className="flex flex-col"
              style={{ gap: `${roundSpacing}px` }}
            >
              <div className="text-[10px] text-text-muted text-center mb-1 whitespace-nowrap">
                {ROUND_NAMES[actualRoundIdx]}
              </div>
              {roundGames.map((game) => (
                <BracketGame
                  key={game.gameIndex}
                  team1={game.team1}
                  team2={game.team2}
                  winner={game.winner}
                  onPick={(pickTeam1) => onPick(game.gameIndex, pickTeam1)}
                  disabled={disabled}
                  compact={actualRoundIdx === 0}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function getVerticalSpacing(round: number): number {
  // Increase spacing as rounds progress so games align with their feeder games
  switch (round) {
    case 0:
      return 2;
    case 1:
      return 28;
    case 2:
      return 82;
    case 3:
      return 190;
    default:
      return 4;
  }
}
