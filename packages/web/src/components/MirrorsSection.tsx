import { useCallback, useEffect, useMemo, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import { BracketMirrorPublicClient } from "@march-madness/client";
import type { MirrorData } from "@march-madness/client";

import { MIRROR_CONTRACT_ADDRESS } from "../lib/constants";

const STORAGE_KEY = "mm-mirrors";

const isZeroAddress = (addr: string) =>
  !addr || addr === "0x0000000000000000000000000000000000000000";

interface StoredMirror {
  mirrorId: number;
  mirror: MirrorData;
  entryCount: number;
}

/** Load mirror IDs from localStorage. */
function loadMirrorIds(): number[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

/** Save mirror IDs to localStorage. */
function saveMirrorIds(ids: number[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(ids));
}

/**
 * Mirrors section — always visible so users can discover and track mirrors.
 */
export function MirrorsSection() {
  const { publicClient } = useShieldedWallet();
  const [mirrors, setMirrors] = useState<StoredMirror[]>([]);
  const [mirrorIds, setMirrorIds] = useState<number[]>(loadMirrorIds);
  const [trackInput, setTrackInput] = useState("");
  const [trackError, setTrackError] = useState<string | null>(null);
  const [isTracking, setIsTracking] = useState(false);

  const hasContract = !isZeroAddress(MIRROR_CONTRACT_ADDRESS);

  const mirrorPublic = useMemo(() => {
    if (!publicClient || !hasContract) return null;
    return new BracketMirrorPublicClient(publicClient, MIRROR_CONTRACT_ADDRESS);
  }, [publicClient, hasContract]);

  // Fetch mirror data whenever mirrorIds change
  useEffect(() => {
    if (!mirrorPublic || mirrorIds.length === 0) {
      setMirrors([]);
      return;
    }
    (async () => {
      const results = await Promise.all(
        mirrorIds.map(async (mirrorId) => {
          try {
            const mirror = await mirrorPublic.getMirror(BigInt(mirrorId));
            const entryCount = await mirrorPublic.getEntryCount(BigInt(mirrorId));
            return { mirrorId, mirror, entryCount: Number(entryCount) };
          } catch {
            return null;
          }
        }),
      );
      setMirrors(results.filter((r): r is StoredMirror => r !== null));
    })();
  }, [mirrorPublic, mirrorIds]);

  const trackMirror = useCallback(
    async (input: string) => {
      if (!mirrorPublic || !input.trim()) return;
      setTrackError(null);
      setIsTracking(true);

      try {
        let mirrorId: number;
        const asNumber = parseInt(input.trim(), 10);

        if (!isNaN(asNumber) && String(asNumber) === input.trim()) {
          // Numeric ID — verify it exists on-chain
          mirrorId = asNumber;
          try {
            const mirror = await mirrorPublic.getMirror(BigInt(mirrorId));
            if (isZeroAddress(mirror.admin)) {
              setTrackError("Mirror not found");
              return;
            }
          } catch {
            setTrackError("Mirror not found");
            return;
          }
        } else {
          // Slug lookup
          try {
            const id = await mirrorPublic.getMirrorBySlug(input.trim());
            mirrorId = Number(id);
            if (mirrorId === 0) {
              setTrackError("Mirror not found for that slug");
              return;
            }
          } catch {
            setTrackError("Mirror not found for that slug");
            return;
          }
        }

        // Check if already tracked
        if (mirrorIds.includes(mirrorId)) {
          setTrackError("Already tracking this mirror");
          return;
        }

        const updated = [...mirrorIds, mirrorId];
        saveMirrorIds(updated);
        setMirrorIds(updated);
        setTrackInput("");
      } catch (err) {
        setTrackError(err instanceof Error ? err.message : "Failed to track mirror");
      } finally {
        setIsTracking(false);
      }
    },
    [mirrorPublic, mirrorIds],
  );

  const untrackMirror = useCallback(
    (mirrorId: number) => {
      const updated = mirrorIds.filter((id) => id !== mirrorId);
      saveMirrorIds(updated);
      setMirrorIds(updated);
    },
    [mirrorIds],
  );

  // Don't render at all if no mirror contract deployed
  if (!hasContract) return null;

  return (
    <div className="rounded-xl bg-bg-secondary border border-border/50 p-4 sm:p-5">
      <h3 className="text-sm font-medium text-text-secondary mb-3">Mirrors</h3>

      {/* Tracked mirrors list */}
      {mirrors.length > 0 ? (
        <div className="space-y-2 mb-3">
          {mirrors.map(({ mirrorId, mirror, entryCount }) => (
            <div
              key={mirrorId}
              className="flex items-center justify-between rounded-lg bg-bg-tertiary border border-border/50 px-3 py-2"
            >
              <div>
                <span className="text-sm text-text-primary">{mirror.displayName}</span>
                <span className="ml-2 text-xs text-text-tertiary">/{mirror.slug}</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-xs text-text-secondary">
                  {entryCount} entr{entryCount !== 1 ? "ies" : "y"}
                </span>
                <button
                  onClick={() => untrackMirror(mirrorId)}
                  className="px-1.5 py-0.5 text-xs rounded bg-bg-primary border border-border/50 text-text-tertiary hover:text-red-400 hover:border-red-800 transition-colors"
                  title="Stop tracking this mirror"
                >
                  Untrack
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-xs text-text-tertiary mb-3">
          No mirrors tracked yet. Track a mirror by ID or slug below.
        </p>
      )}

      {/* Track mirror form */}
      <div className="flex gap-2">
        <input
          type="text"
          value={trackInput}
          onChange={(e) => {
            setTrackInput(e.target.value);
            if (trackError) setTrackError(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && trackInput.trim()) trackMirror(trackInput);
          }}
          placeholder="Mirror ID or slug"
          className="flex-1 px-3 py-1.5 text-sm rounded-lg bg-bg-primary border border-border text-text-primary placeholder:text-text-tertiary"
        />
        <button
          onClick={() => trackMirror(trackInput)}
          disabled={isTracking || !trackInput.trim()}
          className="px-3 py-1.5 text-sm rounded-lg bg-indigo-600 text-white hover:bg-indigo-500 disabled:opacity-50 transition-colors"
        >
          {isTracking ? "..." : "Track"}
        </button>
      </div>
      {trackError && (
        <p className="mt-1 text-xs text-red-400">{trackError}</p>
      )}
    </div>
  );
}
