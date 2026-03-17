import { useState } from "react";

import type { GameStatus } from "@march-madness/client";
import type { Team } from "../lib/tournament";
import { getTeamLogoUrl } from "../lib/espn-logos";

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
    minW = compact ? "min-w-[120px]" : "min-w-[160px]";
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

  return (
    <div
      className={`flex flex-col ${fullWidth ? "w-full min-w-0 rounded-md border border-border/70 bg-bg-primary/20 p-1" : minW} gap-0.5 relative`}
    >
      {/* Live indicator */}
      {gameStatus?.status === "live" && (
        <div className="absolute -top-1 -right-1 flex items-center gap-1 z-10">
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
        isLive={gameStatus?.status === "live"}
        reversed={reversed}
        winProbability={
          gameStatus?.status === "live"
            ? gameStatus.team1WinProbability
            : undefined
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
        isLive={gameStatus?.status === "live"}
        reversed={reversed}
        winProbability={
          gameStatus?.status === "live"
            ? gameStatus.team1WinProbability !== undefined
              ? 1 - gameStatus.team1WinProbability
              : undefined
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
  isLive = false,
  winProbability,
  reversed = false,
}: TeamSlotProps) {
  if (!team) {
    return (
      <div
        className={`${py} ${px} ${textSize} rounded border border-border border-opacity-10 text-text-muted italic text-center `}
      >
        TBD
      </div>
    );
  }

  let className = `${py} ${px} ${textSize} rounded cursor-pointer transition-all border flex items-center justify-between `;

  if (pickCorrect) {
    className +=
      "bg-green-500/15 border-green-500/50 text-text-primary font-semibold";
  } else if (pickWrong) {
    className +=
      "bg-red-500/10 border-red-500/30 text-text-muted line-through opacity-60";
  } else if (isLive && isWinner) {
    className += "bg-accent/20 border-accent text-text-primary font-semibold";
  } else if (isWinner) {
    className += "bg-accent/20 border-accent text-text-primary font-semibold";
  } else if (isLoser) {
    className += "bg-bg-secondary border-border text-text-muted";
  } else {
    className +=
      "bg-bg-tertiary border-border hover:border-accent/50 hover:bg-bg-hover text-text-primary";
  }

  if (disabled) {
    className += " cursor-default opacity-90";
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
          className={`text-text-muted ${mobile ? "w-4" : "w-5"} text-right font-normal flex-shrink-0`}
        >
          {team.seed}
        </span>
        <TeamLogo teamName={team.name} mobile={mobile} />
        <span className="truncate">{team.abbrev ?? team.name}</span>
        {pickCorrect && (
          <span className="ml-1 text-green-400 text-[10px]">&#10003;</span>
        )}
        {pickWrong && (
          <span className="ml-1 text-red-400 text-[10px]">&#10007;</span>
        )}
      </span>
      <span className="flex items-center gap-1">
        {gameScore !== undefined && (
          <span
            className={`font-mono text-[10px] ${isLive ? "text-green-400" : "text-text-muted"}`}
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
