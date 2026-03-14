import type { Team } from "../lib/tournament";

interface BracketGameProps {
  team1: Team | null;
  team2: Team | null;
  winner: Team | null;
  onPick: (pickTeam1: boolean) => void;
  disabled?: boolean;
  compact?: boolean;
  /** Mobile mode — tighter sizing */
  mobile?: boolean;
}

export function BracketGame({
  team1,
  team2,
  winner,
  onPick,
  disabled = false,
  compact = false,
  mobile = false,
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

  return (
    <div className={`flex flex-col ${minW} gap-0.5`}>
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
}: TeamSlotProps) {
  if (!team) {
    return (
      <div
        className={`${py} ${px} ${textSize} rounded border border-dashed border-border-light text-text-muted italic`}
      >
        TBD
      </div>
    );
  }

  let className = `${py} ${px} ${textSize} rounded cursor-pointer transition-all border `;

  if (isWinner) {
    className +=
      "bg-accent/20 border-accent text-text-primary font-semibold";
  } else if (isLoser) {
    className += "bg-bg-secondary border-border text-text-muted";
  } else {
    className +=
      "bg-bg-tertiary border-border hover:border-accent/50 hover:bg-bg-hover text-text-primary";
  }

  if (disabled) {
    className += " cursor-default opacity-70";
  }

  return (
    <button
      className={className}
      onClick={onClick}
      disabled={disabled}
      type="button"
    >
      <span className={`text-text-muted ${mobile ? "mr-0.5" : "mr-1.5"} font-normal`}>
        {team.seed}
      </span>
      <span>{team.abbrev}</span>
    </button>
  );
}
