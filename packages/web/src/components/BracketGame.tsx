import type { Team } from "../lib/tournament";

interface BracketGameProps {
  team1: Team | null;
  team2: Team | null;
  winner: Team | null;
  onPick: (pickTeam1: boolean) => void;
  disabled?: boolean;
  compact?: boolean;
}

export function BracketGame({
  team1,
  team2,
  winner,
  onPick,
  disabled = false,
  compact = false,
}: BracketGameProps) {
  const py = compact ? "py-0.5" : "py-1";
  const px = compact ? "px-2" : "px-3";
  const textSize = compact ? "text-xs" : "text-sm";
  const minW = compact ? "min-w-[120px]" : "min-w-[160px]";

  return (
    <div className={`flex flex-col ${minW} gap-0.5`}>
      <TeamSlot
        team={team1}
        isWinner={winner !== null && winner === team1}
        isLoser={winner !== null && winner !== team1}
        onClick={() => team1 && team2 && !disabled && onPick(true)}
        disabled={disabled || !team1 || !team2}
        py={py}
        px={px}
        textSize={textSize}
      />
      <TeamSlot
        team={team2}
        isWinner={winner !== null && winner === team2}
        isLoser={winner !== null && winner !== team2}
        onClick={() => team1 && team2 && !disabled && onPick(false)}
        disabled={disabled || !team1 || !team2}
        py={py}
        px={px}
        textSize={textSize}
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
      <span className="text-text-muted mr-1.5 font-normal">{team.seed}</span>
      <span>{team.abbrev}</span>
    </button>
  );
}
