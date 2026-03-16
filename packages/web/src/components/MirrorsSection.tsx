import { useEffect, useMemo, useState } from "react";
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

/**
 * Mirrors section — only shown when the user has created/tracked mirrors.
 * Tucked away, not prominent.
 */
export function MirrorsSection() {
  const { publicClient } = useShieldedWallet();
  const [mirrors, setMirrors] = useState<StoredMirror[]>([]);
  const [mirrorIds] = useState<number[]>(loadMirrorIds);

  const hasContract = !isZeroAddress(MIRROR_CONTRACT_ADDRESS);

  const mirrorPublic = useMemo(() => {
    if (!publicClient || !hasContract) return null;
    return new BracketMirrorPublicClient(publicClient, MIRROR_CONTRACT_ADDRESS);
  }, [publicClient, hasContract]);

  useEffect(() => {
    if (!mirrorPublic || mirrorIds.length === 0) return;
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

  // Don't render if no contract or no mirrors tracked
  if (!hasContract || mirrorIds.length === 0 || mirrors.length === 0) return null;

  return (
    <div className="rounded-xl bg-bg-secondary border border-border/50 p-4 sm:p-5">
      <h3 className="text-sm font-medium text-text-secondary mb-3">Mirrors</h3>
      <div className="space-y-2">
        {mirrors.map(({ mirrorId, mirror, entryCount }) => (
          <div
            key={mirrorId}
            className="flex items-center justify-between rounded-lg bg-bg-tertiary border border-border/50 px-3 py-2"
          >
            <div>
              <span className="text-sm text-text-primary">{mirror.displayName}</span>
              <span className="ml-2 text-xs text-text-tertiary">/{mirror.slug}</span>
            </div>
            <span className="text-xs text-text-secondary">
              {entryCount} entr{entryCount !== 1 ? "ies" : "y"}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
