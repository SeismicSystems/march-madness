import { useMemo } from "react";
import { useNavigate, useParams, Link } from "react-router-dom";

import { validateBracket } from "@march-madness/client";

import {
  FinalFourComparison,
  type FinalFourEntry,
} from "../components/FinalFourComparison";
import { useMirror } from "../hooks/useMirror";
import { useMirrorForecasts } from "../hooks/useMirrorForecasts";
import { useTeamProbs } from "../hooks/useTeamProbs";
import { useTournamentStatus } from "../hooks/useTournamentStatus";

export function MirrorFinalFourPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const {
    mirror,
    entries: mirrorEntries,
    loading: mirrorLoading,
    notFound,
    error: mirrorError,
  } = useMirror(id);
  const { status, loading: statusLoading } = useTournamentStatus();
  const { teamProbs } = useTeamProbs();
  const { forecasts } = useMirrorForecasts(id);

  const entries = useMemo(
    (): FinalFourEntry[] =>
      (mirrorEntries ?? []).map((e) => ({
        id: e.slug,
        label: e.slug,
        bracket:
          e.bracket && validateBracket(e.bracket)
            ? (e.bracket as `0x${string}`)
            : null,
      })),
    [mirrorEntries],
  );

  if (notFound || mirrorError) {
    return (
      <div className="max-w-xl mx-auto text-center py-12">
        <h2 className="text-lg font-bold text-text-primary mb-2">
          Mirror not found
        </h2>
        <p className="text-text-muted mb-4">
          {mirrorError ?? `Mirror with ID "${id}" was not found.`}
        </p>
      </div>
    );
  }

  if (mirrorLoading || statusLoading) {
    return (
      <div className="text-center py-12 text-text-muted">Loading...</div>
    );
  }

  const displayTitle = mirror?.display_name ?? `Mirror ${id}`;

  return (
    <FinalFourComparison
      entries={entries}
      title="Final Four"
      backLink={{ to: `/mirrors/id/${id}`, label: displayTitle }}
      tournamentStatus={status}
      teamProbs={teamProbs}
      onEntryClick={(entry) =>
        navigate(`/mirrors/id/${id}/bracket/${entry.id}`)
      }
      orderKey={mirror?.slug}
      forecasts={forecasts}
    />
  );
}
