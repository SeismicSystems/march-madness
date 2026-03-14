interface ScoreboardProps {
  entryCount: number;
}

export function Scoreboard({ entryCount }: ScoreboardProps) {
  return (
    <div className="bg-bg-secondary border border-border rounded-xl p-4 sm:p-8 text-center">
      <h2 className="text-base sm:text-lg font-semibold text-text-primary mb-2">
        Scoreboard
      </h2>
      <p className="text-text-secondary text-xs sm:text-sm mb-4">
        Scores will appear here once tournament results are posted and brackets
        are scored.
      </p>
      <div className="grid grid-cols-3 gap-2 sm:gap-4 max-w-sm mx-auto">
        <div className="bg-bg-tertiary rounded-lg p-2 sm:p-3 border border-border">
          <div className="text-xl sm:text-2xl font-bold text-text-primary">
            {entryCount}
          </div>
          <div className="text-[10px] sm:text-xs text-text-muted">Entries</div>
        </div>
        <div className="bg-bg-tertiary rounded-lg p-2 sm:p-3 border border-border">
          <div className="text-xl sm:text-2xl font-bold text-text-muted">--</div>
          <div className="text-[10px] sm:text-xs text-text-muted">Top Score</div>
        </div>
        <div className="bg-bg-tertiary rounded-lg p-2 sm:p-3 border border-border">
          <div className="text-xl sm:text-2xl font-bold text-text-muted">--</div>
          <div className="text-[10px] sm:text-xs text-text-muted">Winners</div>
        </div>
      </div>
    </div>
  );
}
