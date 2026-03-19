import { useState } from "react";

import type { GameStatus } from "@march-madness/client";
import { displayAbbrev, displayName, type Team } from "../lib/tournament";
import { getTeamLogoUrl } from "../lib/espn-logos";

/** Format seconds as M:SS (e.g. 450 => "7:30", 75 => "1:15"). */
function formatClock(totalSeconds: number): string {
  const s = Math.max(0, totalSeconds);
  const min = Math.floor(s / 60);
  const sec = s % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

/** Format period + seconds into a compact label like "1H 7:30", "2H 1:15", "OT 3:00". */
function formatPeriodClock(period: number, secondsRemaining: number): string {
  const clock = formatClock(secondsRemaining);
  if (period === 1) return `1H ${clock}`;
  if (period === 2) return `2H ${clock}`;
  if (period === 3) return `OT ${clock}`;
  return `${period - 2}OT ${clock}`;
}

interface BracketGameProps {
  team1: Team | null;
  team2: Team | null;
  winner: Team | null;
  onPick: (pickTeam1: boolean) => void;
  disabled?: boolean;
  compact?: boolean;
  /** Mobile mode — tighter sizing */
  mobile?: boolean;
  /** Tournament status overlay for this game */
  gameStatus?: GameStatus;
  /** Whether this region reads right-to-left (logos on right side) */
  reversed?: boolean;
  /** Stretch game card to its container width (used by mobile stacked lanes). */
  fullWidth?: boolean;
  /** Round number (0-based: 0=R64, 1=R32, ..., 4=F4, 5=Championship) */
  round?: number;
  /** Teams that have been eliminated from the tournament */
  eliminatedTeams?: Set<string>;
  /** Teams still alive: name → number of tournament wins */
  advancedTeams?: Map<string, number>;
  /** Externally computed win probability for team1 (0-1), derived from team advance probs. */
  team1WinProbability?: number;
}

export function BracketGame({
  team1,
  team2,
  winner,
  onPick,
  disabled = false,
  compact = false,
  mobile = false,
  gameStatus,
  reversed = false,
  fullWidth = false,
  round = 0,
  eliminatedTeams,
  advancedTeams,
  team1WinProbability,
}: BracketGameProps) {
  let py: string, px: string, textSize: string, minW: string;

  if (mobile) {
    py = "py-0.5";
    px = "px-1.5";
    textSize = "text-[11px]";
    minW = compact ? "min-w-[72px]" : "min-w-[80px]";
  } else {
    py = compact ? "py-0.5" : "py-1";
    px = compact ? "px-2" : "px-3";
    textSize = compact ? "text-xs" : "text-sm";
    minW = "w-full min-w-0";
  }

  // Determine if each team's pick was correct/wrong based on game result
  const pickCorrectTeam1 =
    gameStatus?.status === "final" &&
    gameStatus.winner === true &&
    winner === team1;
  const pickCorrectTeam2 =
    gameStatus?.status === "final" &&
    gameStatus.winner === false &&
    winner === team2;
  const pickWrongTeam1 =
    gameStatus?.status === "final" &&
    gameStatus.winner === false &&
    winner === team1;
  const pickWrongTeam2 =
    gameStatus?.status === "final" &&
    gameStatus.winner === true &&
    winner === team2;

  // "Busted" = user picked this team but it was already eliminated in a prior round.
  // Only applies when the team is the user's pick (isWinner) and not already shown as pickWrong.
  const eliminatedTeam1 =
    !pickWrongTeam1 &&
    !pickCorrectTeam1 &&
    winner === team1 &&
    team1 !== null &&
    !!eliminatedTeams?.has(displayName(team1));
  const eliminatedTeam2 =
    !pickWrongTeam2 &&
    !pickCorrectTeam2 &&
    winner === team2 &&
    team2 !== null &&
    !!eliminatedTeams?.has(displayName(team2));

  // "Advancing" = user picked this team and the team has enough wins to have
  // actually reached this round. A team needs at least `round` wins to be here.
  const advancingTeam1 =
    !pickCorrectTeam1 &&
    !pickWrongTeam1 &&
    !eliminatedTeam1 &&
    winner === team1 &&
    team1 !== null &&
    (advancedTeams?.get(displayName(team1)) ?? -1) >= round;
  const advancingTeam2 =
    !pickCorrectTeam2 &&
    !pickWrongTeam2 &&
    !eliminatedTeam2 &&
    winner === team2 &&
    team2 !== null &&
    (advancedTeams?.get(displayName(team2)) ?? -1) >= round;

  return (
    <div
      className={`flex flex-col ${
        fullWidth
          ? "w-full min-w-0 rounded-md border border-border/70 bg-bg-primary/20 p-1"
          : minW
      } gap-0.5 relative`}
    >
      {/* Live indicator with period/clock */}
      {gameStatus?.status === "live" && (
        <div className="absolute -top-1 -right-1 flex items-center gap-1 z-10">
          {gameStatus.period != null && gameStatus.secondsRemaining != null && (
            <span className="text-[8px] text-green-400 font-mono leading-none">
              {formatPeriodClock(
                gameStatus.period,
                gameStatus.secondsRemaining
              )}
            </span>
          )}
          <span className="relative flex h-2 w-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
            <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
          </span>
        </div>
      )}
      <TeamSlot
        team={team1}
        isWinner={winner !== null && winner === team1}
        isLoser={winner !== null && winner !== team1 && team1 !== null}
        onClick={() => team1 && !disabled && onPick(true)}
        disabled={disabled || !team1}
        py={py}
        px={px}
        textSize={textSize}
        mobile={mobile}
        gameScore={gameStatus?.score?.team1}
        pickCorrect={pickCorrectTeam1}
        pickWrong={pickWrongTeam1}
        isEliminated={eliminatedTeam1}
        isAdvancing={advancingTeam1}
        isLive={gameStatus?.status === "live"}
        reversed={reversed}
        winProbability={
          gameStatus?.status !== "final" ? team1WinProbability : undefined
        }
      />
      <TeamSlot
        team={team2}
        isWinner={winner !== null && winner === team2}
        isLoser={winner !== null && winner !== team2 && team2 !== null}
        onClick={() => team2 && !disabled && onPick(false)}
        disabled={disabled || !team2}
        py={py}
        px={px}
        textSize={textSize}
        mobile={mobile}
        gameScore={gameStatus?.score?.team2}
        pickCorrect={pickCorrectTeam2}
        pickWrong={pickWrongTeam2}
        isEliminated={eliminatedTeam2}
        isAdvancing={advancingTeam2}
        isLive={gameStatus?.status === "live"}
        reversed={reversed}
        winProbability={
          gameStatus?.status !== "final" && team1WinProbability !== undefined
            ? 1 - team1WinProbability
            : undefined
        }
      />
    </div>
  );
}

interface TeamSlotProps {
  team: Team | null;
  isWinner: boolean;
  isLoser: boolean;
  onClick: () => void;
  disabled: boolean;
  py: string;
  px: string;
  textSize: string;
  mobile?: boolean;
  gameScore?: number;
  pickCorrect?: boolean;
  pickWrong?: boolean;
  isEliminated?: boolean;
  isAdvancing?: boolean;
  isLive?: boolean;
  winProbability?: number;
  reversed?: boolean;
}

function TeamSlot({
  team,
  isWinner,
  isLoser,
  onClick,
  disabled,
  py,
  px,
  textSize,
  mobile = false,
  gameScore,
  pickCorrect = false,
  pickWrong = false,
  isEliminated = false,
  isAdvancing = false,
  isLive = false,
  winProbability,
  reversed = false,
}: TeamSlotProps) {
  if (!team) {
    return (
      <div
        className={`${py} ${px} ${textSize} rounded-lg border border-border/30 backdrop-blur-md bg-bg-secondary/30 text-text-muted italic text-center`}
      >
        TBD
      </div>
    );
  }

  let className = `${py} ${px} ${textSize} rounded-lg ${disabled ? "cursor-default" : "cursor-pointer"} transition-all border backdrop-blur-md flex items-center justify-between `;

  if (pickCorrect) {
    className +=
      "bg-green-500/10 border-green-500/40 text-text-primary font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]";
  } else if (pickWrong) {
    className +=
      "bg-red-500/10 border-red-500/25 text-text-muted line-through opacity-60";
  } else if (isEliminated) {
    className +=
      "bg-red-500/10 border-red-500/25 text-red-400/80 font-semibold";
  } else if (isAdvancing) {
    className +=
      "bg-green-500/10 border-green-500/40 text-text-primary font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]";
  } else if (isLive && isWinner) {
    className +=
      "bg-accent/15 border-accent/60 text-text-primary font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]";
  } else if (isWinner) {
    className +=
      "bg-accent/15 border-accent/60 text-text-primary font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]";
  } else if (isLoser) {
    className += "bg-bg-secondary/40 border-border/30 text-text-muted";
  } else {
    className +=
      "bg-bg-tertiary/40 border-border/30 hover:border-accent/40 hover:bg-bg-hover/50 text-text-primary shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]";
  }

  if (disabled) {
    className += " opacity-90";
  }

  return (
    <button
      className={className}
      onClick={onClick}
      disabled={disabled}
      type="button"
    >
      <span className="flex items-center gap-2 min-w-0">
        <span
          className={`text-text-muted ${
            mobile ? "w-4" : "w-5"
          } text-right font-normal flex-shrink-0`}
        >
          {team.seed}
        </span>
        <TeamLogo teamName={displayName(team)} mobile={mobile} />
        <span className="truncate">{displayAbbrev(team)}</span>
        {pickCorrect && (
          <span className="ml-1 text-green-400 text-[10px]">&#10003;</span>
        )}
        {pickWrong && (
          <span className="ml-1 text-red-400 text-[10px]">&#10007;</span>
        )}
        {isAdvancing && (
          <span className="ml-1 text-green-400 text-[10px]">&#10003;</span>
        )}
        {isEliminated && (
          <span className="ml-1 text-red-400 text-[10px]">&#10007;</span>
        )}
      </span>
      <span className="flex items-center gap-1">
        {gameScore !== undefined && (
          <span
            className={`font-mono text-[10px] ${
              isLive ? "text-green-400" : "text-text-muted"
            }`}
          >
            {gameScore}
          </span>
        )}
        {isLive && winProbability !== undefined && (
          <span className="text-[9px] text-text-muted bg-bg-secondary/80 px-1 rounded">
            {Math.round(winProbability * 100)}%
          </span>
        )}
      </span>
    </button>
  );
}

export function TeamLogo({
  teamName,
  mobile = false,
}: {
  teamName: string;
  mobile?: boolean;
}) {
  const [failed, setFailed] = useState(false);
  const url = getTeamLogoUrl(teamName);
  const size = mobile ? "w-3 h-3" : "w-6 h-6";
  return (
    <div className={`${size} flex-shrink-0`}>
      {url && !failed && (
        <img
          src={url}
          alt=""
          className="w-full h-full object-contain"
          onError={() => setFailed(true)}
          loading="lazy"
        />
      )}
    </div>
  );
}
