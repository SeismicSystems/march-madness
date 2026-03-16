import type { TournamentStatus } from "@march-madness/client";

import type { GameSlot } from "../hooks/useBracket";
import { BracketGame } from "./BracketGame";

interface FinalFourProps {
  semifinal1: GameSlot | null;
  semifinal2: GameSlot | null;
  championship: GameSlot | null;
  onPick: (gameIndex: number, pickTeam1: boolean) => void;
  disabled?: boolean;
  tournamentStatus?: TournamentStatus;
}

export function FinalFour({
  semifinal1,
  semifinal2,
  championship,
  onPick,
  disabled = false,
  tournamentStatus,
}: FinalFourProps) {
  return (
    <div className="flex flex-col items-center min-w-[200px]">
      <h3 className="text-sm font-semibold text-gold uppercase tracking-wider mb-3 px-1">
        Final Four
      </h3>
      {/* Spacer matching the round-label row in BracketRegion */}
      <div className="text-[10px] mb-1 invisible">​</div>

      <div className="flex flex-col items-center justify-center gap-8 flex-1">
        {/* Semifinal 1 */}
        {semifinal1 && (
          <BracketGame
            team1={semifinal1.team1}
            team2={semifinal1.team2}
            winner={semifinal1.winner}
            onPick={(pickTeam1) => onPick(semifinal1.gameIndex, pickTeam1)}
            disabled={disabled}
            gameStatus={tournamentStatus?.games[semifinal1.gameIndex]}
          />
        )}

        {/* Championship */}
        <div className="flex flex-col items-center gap-2">
          <div className="text-[10px] text-text-muted uppercase tracking-wider">
            Championship
          </div>
          {championship && (
            <BracketGame
              team1={championship.team1}
              team2={championship.team2}
              winner={championship.winner}
              onPick={(pickTeam1) => onPick(championship.gameIndex, pickTeam1)}
              disabled={disabled}
              gameStatus={tournamentStatus?.games[championship.gameIndex]}
            />
          )}
          {championship?.winner && (
            <div className="mt-2 px-4 py-2 bg-gold/20 border border-gold/50 rounded-lg text-center">
              <div className="text-[10px] text-gold/80 uppercase">Champion</div>
              <div className="text-lg font-bold text-gold">
                {championship.winner.seed} {championship.winner.name}
              </div>
            </div>
          )}
        </div>

        {/* Semifinal 2 */}
        {semifinal2 && (
          <BracketGame
            team1={semifinal2.team1}
            team2={semifinal2.team2}
            winner={semifinal2.winner}
            onPick={(pickTeam1) => onPick(semifinal2.gameIndex, pickTeam1)}
            disabled={disabled}
            gameStatus={tournamentStatus?.games[semifinal2.gameIndex]}
          />
        )}
      </div>
    </div>
  );
}
