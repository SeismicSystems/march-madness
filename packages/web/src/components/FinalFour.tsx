import type { TournamentStatus } from "@march-madness/client";

import type { GameSlot } from "../hooks/useBracket";
import type { ActualTeamMap, GameWinProbs } from "./BracketView";
import { displayName } from "../lib/tournament";
import { BracketGame, TeamLogo } from "./BracketGame";

interface FinalFourProps {
  semifinal1: GameSlot | null;
  semifinal2: GameSlot | null;
  championship: GameSlot | null;
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled?: boolean;
  tournamentStatus?: TournamentStatus;
  eliminatedTeams?: Set<string>;
  advancedTeams?: Map<string, number>;
  gameWinProbs?: GameWinProbs;
  actualTeams?: ActualTeamMap | null;
}

export function FinalFour({
  semifinal1,
  semifinal2,
  championship,
  onPick,
  disabled = false,
  tournamentStatus,
  eliminatedTeams,
  advancedTeams,
  gameWinProbs,
  actualTeams,
}: FinalFourProps) {
  return (
    <div className="flex flex-col items-center min-w-0">
      <h3 className="text-sm font-semibold  uppercase tracking-wider mb-3 px-1">
        Final Four
      </h3>
      {/* Spacer matching the round-label row in BracketRegion */}
      <div className="text-[10px] mb-1 invisible">​</div>

      <div className="flex flex-col items-center justify-center gap-6 flex-1 w-full">
        <div className="flex flex-col md:flex-row gap-6 md:gap-8 items-center md:items-start justify-center">
          {semifinal1 && (() => {
            const at = actualTeams?.get(semifinal1.gameIndex);
            return (
              <div className="w-[180px]">
                <BracketGame
                  team1={semifinal1.team1}
                  team2={semifinal1.team2}
                  winner={semifinal1.winner}
                  onPick={(pickTeam1) => onPick(semifinal1.gameIndex, pickTeam1)}
                  disabled={disabled}
                  round={4}
                  gameStatus={tournamentStatus?.games[semifinal1.gameIndex]}
                  eliminatedTeams={eliminatedTeams}
                  advancedTeams={advancedTeams}
                  team1WinProbability={gameWinProbs?.get(semifinal1.gameIndex)}
                  actualTeam1={at?.team1}
                  actualTeam2={at?.team2}
                />
              </div>
            );
          })()}
          {semifinal2 && (() => {
            const at = actualTeams?.get(semifinal2.gameIndex);
            return (
              <div className="w-[180px]">
                <BracketGame
                  team1={semifinal2.team1}
                  team2={semifinal2.team2}
                  winner={semifinal2.winner}
                  onPick={(pickTeam1) => onPick(semifinal2.gameIndex, pickTeam1)}
                  disabled={disabled}
                  round={4}
                  gameStatus={tournamentStatus?.games[semifinal2.gameIndex]}
                  eliminatedTeams={eliminatedTeams}
                  advancedTeams={advancedTeams}
                  team1WinProbability={gameWinProbs?.get(semifinal2.gameIndex)}
                  actualTeam1={at?.team1}
                  actualTeam2={at?.team2}
                />
              </div>
            );
          })()}
        </div>

        <div className="flex flex-col items-center gap-2 md:mt-4">
          <div className="text-md  text-gold  uppercase tracking-wider">
            Championship
          </div>
          {championship && (() => {
            const at = actualTeams?.get(championship.gameIndex);
            return (
              <div className="w-[240px]">
                <BracketGame
                  team1={championship.team1}
                  team2={championship.team2}
                  winner={championship.winner}
                  onPick={(pickTeam1) =>
                    onPick(championship.gameIndex, pickTeam1)
                  }
                  disabled={disabled}
                  round={5}
                  gameStatus={tournamentStatus?.games[championship.gameIndex]}
                  eliminatedTeams={eliminatedTeams}
                  advancedTeams={advancedTeams}
                  team1WinProbability={gameWinProbs?.get(championship.gameIndex)}
                  actualTeam1={at?.team1}
                  actualTeam2={at?.team2}
                />
              </div>
            );
          })()}
          {championship?.winner && (
            <div className="mt-2 px-4 py-2 bg-gold/20 border border-gold/50 rounded-lg text-center">
              <div className="text-[10px] text-gold/80 uppercase">Champion</div>
              <div className="text-lg font-bold text-gold flex items-center justify-center gap-2">
                <TeamLogo teamName={displayName(championship.winner)} />
                {championship.winner.seed} {displayName(championship.winner)}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
