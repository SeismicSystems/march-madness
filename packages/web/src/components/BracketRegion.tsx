import type { TournamentStatus } from "@march-madness/client";

import type { GameSlot } from "../hooks/useBracket";
import type { ActualTeamMap, GameWinProbs } from "./BracketView";
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
  tournamentStatus?: TournamentStatus;
  eliminatedTeams?: Set<string>;
  advancedTeams?: Map<string, number>;
  gameWinProbs?: GameWinProbs;
  actualTeams?: ActualTeamMap | null;
}

export function BracketRegion({
  regionName,
  rounds,
  onPick,
  disabled = false,
  reversed = false,
  compact = false,
  tournamentStatus,
  eliminatedTeams,
  advancedTeams,
  gameWinProbs,
  actualTeams,
}: BracketRegionProps) {
  const orderedRounds = reversed ? [...rounds].reverse() : rounds;

  return (
    <div className="flex flex-col flex-1 min-w-0">
      <h3
        className={`text-sm font-semibold text-accent uppercase tracking-wider mb-3 px-1 ${
          reversed ? "text-right" : ""
        }`}
      >
        {regionName}
      </h3>
      <div className="flex flex-row items-stretch gap-1 flex-1 min-w-0">
        {orderedRounds.map((roundGames, displayIdx) => {
          const actualRoundIdx = reversed
            ? rounds.length - 1 - displayIdx
            : displayIdx;

          return (
            <div key={displayIdx} className="flex flex-col flex-1 min-w-0">
              {/* <div className="text-[10px] text-text-muted text-center mb-1 whitespace-nowrap">
                {ROUND_NAMES[actualRoundIdx]}
              </div> */}
              <div className="flex flex-col flex-1 justify-around gap-4 min-w-0">
                {roundGames.map((game) => {
                  const at = actualTeams?.get(game.gameIndex);
                  return (
                    <BracketGame
                      key={game.gameIndex}
                      team1={game.team1}
                      team2={game.team2}
                      winner={game.winner}
                      onPick={(pickTeam1) => onPick(game.gameIndex, pickTeam1)}
                      disabled={disabled}
                      compact={actualRoundIdx === 0}
                      mobile={compact}
                      reversed={reversed}
                      round={actualRoundIdx}
                      gameStatus={tournamentStatus?.games[game.gameIndex]}
                      eliminatedTeams={eliminatedTeams}
                      advancedTeams={advancedTeams}
                      team1WinProbability={gameWinProbs?.get(game.gameIndex)}
                      actualTeam1={at?.team1}
                      actualTeam2={at?.team2}
                    />
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
