import { usePrivy } from "@privy-io/react-auth";

import { truncateAddress } from "../lib/tournament";

interface HeaderProps {
  entryCount: number;
}

export function Header({ entryCount }: HeaderProps) {
  const { login, logout, authenticated, user } = usePrivy();

  const address = user?.wallet?.address;

  return (
    <header className="border-b border-border bg-bg-secondary/80 backdrop-blur-sm sticky top-0 z-50">
      <div className="max-w-[1800px] mx-auto px-3 sm:px-4 py-2 sm:py-3 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 sm:gap-4 min-w-0">
          <h1 className="text-base sm:text-xl font-bold text-text-primary tracking-tight whitespace-nowrap">
            March Madness
            <span className="text-accent ml-1 sm:ml-2 font-normal text-xs sm:text-sm">
              on Seismic
            </span>
          </h1>
          {entryCount > 0 && (
            <span className="hidden sm:inline text-xs text-text-muted bg-bg-tertiary px-2 py-1 rounded-full">
              {entryCount} {entryCount === 1 ? "entry" : "entries"}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
          {authenticated && address && (
            <span className="hidden sm:inline text-sm text-text-secondary font-mono">
              {truncateAddress(address)}
            </span>
          )}
          {authenticated ? (
            <button
              onClick={logout}
              className="px-3 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm rounded-lg bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors border border-border"
            >
              Disconnect
            </button>
          ) : (
            <button
              onClick={login}
              className="px-3 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm rounded-lg bg-accent text-white hover:bg-accent-hover transition-colors font-medium"
            >
              Connect
            </button>
          )}
        </div>
      </div>
    </header>
  );
}
