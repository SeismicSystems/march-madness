import { Link } from "react-router-dom";

import { Card, CardContent } from "@/components/ui/card";
import type { BracketForecast, PartialScore } from "@march-madness/client";

import { TeamLogo } from "./BracketGame";
import { truncateAddress } from "../lib/tournament";

export interface LeaderboardEntry {
  address: string;
  tag?: string;
  bracket: `0x${string}` | null;
  score: PartialScore | null;
  championName: string | null;
  forecast?: BracketForecast;
  sortLabel: string;
}

/** Brand-colored radial gradient background, brighter for top-3. */
function gradientStyle(rank: number) {
  const intensity = rank <= 3 ? 0.35 : 0.15;
  return {
    backgroundImage: `
      radial-gradient(ellipse at 20% 30%, rgba(130, 90, 109, ${intensity}) 0%, transparent 60%),
      radial-gradient(ellipse at 80% 70%, rgba(82, 53, 66, ${
        intensity * 0.8
      }) 0%, transparent 70%),
      radial-gradient(ellipse at 60% 20%, rgba(166, 146, 77, ${
        intensity * 0.5
      }) 0%, transparent 50%)
    `,
  };
}

function CardInner({
  entry,
  rank,
  hasForecasts,
}: {
  entry: LeaderboardEntry;
  rank: number;
  hasForecasts: boolean;
}) {
  return (
    <div className="relative overflow-hidden rounded-2xl bg-bg-secondary hover:bg-bg-hover/50 transition-colors">
      <div
        className="absolute inset-0 rounded-2xl"
        style={gradientStyle(rank)}
      />
      <Card size="sm" className="z-10 isolate bg-transparent border-border/50">
        <CardContent>
          <div className="flex items-center gap-4">
            {/* Rank */}
            <span className="text-text-muted font-mono text-lg w-10 shrink-0">
              #{rank}
            </span>

            {/* Player */}
            <div className="min-w-0 flex-1">
              {entry.tag ? (
                <>
                  <div className="text-text-primary font-medium truncate">
                    {entry.tag}
                  </div>
                  <div className="text-[10px] text-text-muted font-mono">
                    {truncateAddress(entry.address)}
                  </div>
                </>
              ) : (
                <div className="text-text-primary font-mono text-sm">
                  {truncateAddress(entry.address)}
                </div>
              )}
            </div>

            {/* Champion */}
            {entry.championName ? (
              <div className="flex items-center gap-1.5 shrink-0">
                <TeamLogo teamName={entry.championName} />
                <span className="text-text-secondary text-xs">
                  {entry.championName}
                </span>
              </div>
            ) : (
              <span className="text-text-muted text-xs shrink-0">—</span>
            )}

            {/* Forecast stats */}
            {hasForecasts && entry.forecast && (
              <div className="text-right text-xs hidden md:block shrink-0 w-24">
                <div className="text-text-secondary">
                  E[Score]: {entry.forecast.expectedScore.toFixed(1)}
                </div>
                <div
                  className={
                    entry.forecast.winProbability > 0.1
                      ? "text-green-400 font-medium"
                      : "text-text-muted"
                  }
                >
                  P(Win): {(entry.forecast.winProbability * 100).toFixed(1)}%
                </div>
              </div>
            )}

            {/* Score */}
            <div className="text-right shrink-0">
              {entry.score ? (
                <>
                  <span className="text-text-primary font-bold text-lg">
                    {entry.score.current}
                  </span>
                  <span className="text-text-muted text-sm">
                    /{entry.score.maxPossible}
                  </span>
                </>
              ) : (
                <span className="text-text-muted">—</span>
              )}
            </div>

            {/* Arrow */}
            {entry.bracket && (
              <span className="text-accent group-hover/link:text-accent-hover transition-colors shrink-0">
                &rarr;
              </span>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export function LeaderboardCard({
  entry,
  rank,
  hasForecasts,
}: {
  entry: LeaderboardEntry;
  rank: number;
  hasForecasts: boolean;
}) {
  if (entry.bracket) {
    return (
      <Link to={`/bracket/${entry.address}`} className="block group/link">
        <CardInner entry={entry} rank={rank} hasForecasts={hasForecasts} />
      </Link>
    );
  }
  return <CardInner entry={entry} rank={rank} hasForecasts={hasForecasts} />;
}
