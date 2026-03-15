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
  /** Compact mode for mobile — smaller sizing and spacing */
  compact?: boolean;
}

export function BracketRegion({
  regionName,
  rounds,
  onPick,
  disabled = false,
  reversed = false,
  compact = false,
}: BracketRegionProps) {
  const orderedRounds = reversed ? [...rounds].reverse() : rounds;

  return (
    <div className="flex flex-col">
      <h3 className={`text-sm font-semibold text-accent uppercase tracking-wider mb-3 px-1 ${reversed ? "text-right" : ""}`}>
        {regionName}
      </h3>
      <div className="flex flex-row items-center gap-1">
        {orderedRounds.map((roundGames, displayIdx) => {
          const actualRoundIdx = reversed
            ? rounds.length - 1 - displayIdx
            : displayIdx;
          const roundSpacing = getVerticalSpacing(actualRoundIdx, compact);

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
                  mobile={compact}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function getVerticalSpacing(round: number, compact: boolean): number {
  if (compact) {
    // Scaled-down spacing for mobile
    switch (round) {
      case 0: return 6;
      case 1: return 22;
      case 2: return 56;
      case 3: return 124;
      default: return 6;
    }
  }
  switch (round) {
    case 0: return 8;
    case 1: return 32;
    case 2: return 86;
    case 3: return 194;
    default: return 8;
  }
}
