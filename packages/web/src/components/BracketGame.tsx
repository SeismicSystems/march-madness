import { useState } from "react";

import type { GameStatus } from "@march-madness/client";
import { displayAbbrev, displayName, tournament, type Team } from "../lib/tournament";
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
  // Halftime: end of 1st half with no time left
  if (period === 1 && secondsRemaining === 0) return "HALF";
  const clock = formatClock(secondsRemaining);
  if (period === 1) return `1H ${clock}`;
  if (period === 2) return `2H ${clock}`;
  if (period === 3) return `OT ${clock}`;
  return `${period - 2}OT ${clock}`;
}

/** Banner shown above team slots when a game is live. */
function LiveBanner({ gameStatus }: { gameStatus: GameStatus }) {
  const bracketId = tournament.bracketIds[gameStatus.gameIndex];
  const watchUrl = `https://www.ncaa.com/march-madness-live/game/${bracketId}`;

  const clockLabel =
    gameStatus.period != null && gameStatus.secondsRemaining != null
      ? formatPeriodClock(gameStatus.period, gameStatus.secondsRemaining)
      : "LIVE";

  return (
    <div className="flex items-center justify-between px-2 py-0.5 rounded-t-md bg-green-500/10 border border-green-500/25 text-[10px]">
      <span className="font-mono font-semibold text-green-400">{clockLabel}</span>
      <span className="flex items-center gap-1.5">
        <a
          href={watchUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-green-400/80 underline decoration-[0.5px] underline-offset-2 hover:text-green-300"
          onClick={(e) => e.stopPropagation()}
        >
          Watch
        </a>
        <span className="relative flex h-2 w-2">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
          <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
        </span>
      </span>
    </div>
  );
}

/**
 * Tournament overlay state for a single team slot. Determined once per team,
 * then used for both styling and iconography — no interleaved boolean priority.
 */
const enum Overlay {
  /** Game final, user's pick won. */
  Correct = 1,
  /** Game final, user's pick lost. */
  Wrong,
  /** User picked this team but it lost in a prior round. */
  Eliminated,
  /** User picked this team and it has won THROUGH this round. */
  Advancing,
}

function computeOverlay(
  team: Team | null,
  isUserPick: boolean,
  isTeam1: boolean,
  gameStatus: GameStatus | undefined,
  eliminatedTeams: Set<string> | undefined,
  advancedTeams: Map<string, number> | undefined,
  round: number,
): Overlay | null {
  if (!team || !isUserPick) return null;

  // Game has a final result — was the user's pick right or wrong?
  if (gameStatus?.status === "final" && gameStatus.winner !== undefined) {
    const pickedTeamWon = isTeam1 ? gameStatus.winner : !gameStatus.winner;
    return pickedTeamWon ? Overlay.Correct : Overlay.Wrong;
  }

  // Game not decided yet — was this team already knocked out in a prior round?
  const name = displayName(team);
  if (eliminatedTeams?.has(name)) return Overlay.Eliminated;

  // Team is still alive and has won THROUGH this round (wins > round).
  // wins == round only means they've arrived at this round, not cleared it.
  if ((advancedTeams?.get(name) ?? -1) > round) return Overlay.Advancing;

  return null;
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

  const overlayTeam1 = computeOverlay(team1, winner === team1, true, gameStatus, eliminatedTeams, advancedTeams, round);
  const overlayTeam2 = computeOverlay(team2, winner === team2, false, gameStatus, eliminatedTeams, advancedTeams, round);

  return (
    <div
      className={`flex flex-col ${
        fullWidth
          ? "w-full min-w-0 rounded-md border border-border/70 bg-bg-primary/20 p-1"
          : minW
      } gap-0.5`}
    >
      {gameStatus?.status === "live" && <LiveBanner gameStatus={gameStatus} />}
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
        overlay={overlayTeam1}
        isLive={gameStatus?.status === "live"}
        reversed={reversed}
        winProbability={
          gameStatus?.status !== "final" &&
          !(team1 && eliminatedTeams?.has(displayName(team1)))
            ? team1WinProbability
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
        overlay={overlayTeam2}
        isLive={gameStatus?.status === "live"}
        reversed={reversed}
        winProbability={
          gameStatus?.status !== "final" &&
          team1WinProbability !== undefined &&
          !(team2 && eliminatedTeams?.has(displayName(team2)))
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
  overlay: Overlay | null;
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
  overlay,
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

  if (overlay === Overlay.Correct) {
    className +=
      "bg-green-500/10 border-green-500/40 text-text-primary font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]";
  } else if (overlay === Overlay.Wrong) {
    className +=
      "bg-red-500/10 border-red-500/25 text-text-muted line-through opacity-60";
  } else if (overlay === Overlay.Eliminated) {
    className +=
      "bg-red-500/10 border-red-500/25 text-red-400/80 font-semibold";
  } else if (overlay === Overlay.Advancing) {
    className +=
      "bg-accent/15 border-green-500/25 text-text-primary font-semibold shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]";
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
      <span className="flex items-center gap-1.5 min-w-0">
        <span
          className={`text-text-muted ${
            mobile ? "w-3.5" : "w-[17px]"
          } text-right font-normal flex-shrink-0`}
        >
          {team.seed}
        </span>
        <TeamLogo teamName={displayName(team)} mobile={mobile} />
        <span className="truncate">{displayAbbrev(team)}</span>
      </span>
      <span className="flex items-center gap-1 flex-shrink-0">
        {winProbability !== undefined && (
          <span className={`text-[9px] bg-bg-secondary/80 px-1 rounded ${isLive ? "text-text-muted" : "text-text-muted/70"}`}>
            {Math.round(winProbability * 100)}%
          </span>
        )}
        {gameScore !== undefined && (
          <span
            className={`font-mono text-[10px] ${
              isLive ? "text-green-400 font-semibold" : "text-text-muted"
            }`}
          >
            {gameScore}
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
