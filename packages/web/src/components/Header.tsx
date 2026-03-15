import { useRef, useState, useEffect } from "react";
import { usePrivy } from "@privy-io/react-auth";

import { FAUCET_URL } from "../lib/constants";
import { truncateAddress } from "../lib/tournament";
import { useIsMobile } from "../hooks/useIsMobile";

interface HeaderProps {
  entryCount: number;
}

export function Header({ entryCount }: HeaderProps) {
  const { login, logout, authenticated, user } = usePrivy();
  const [copied, setCopied] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const isMobile = useIsMobile();

  const address = user?.wallet?.address;

  const copyAddress = () => {
    if (!address) return;
    navigator.clipboard.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Close menu on outside click
  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [menuOpen]);

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
          {entryCount > 0 && !isMobile && (
            <span className="text-xs text-text-muted bg-bg-tertiary px-2 py-1 rounded-full">
              {entryCount} {entryCount === 1 ? "entry" : "entries"}
            </span>
          )}
        </div>

        {/* Desktop: inline buttons */}
        {!isMobile && (
          <div className="flex items-center gap-3 shrink-0">
            <a
              href={FAUCET_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="px-3 py-2 text-sm rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors"
            >
              Faucet
            </a>
            {authenticated && address && (
              <button
                onClick={copyAddress}
                type="button"
                title="Copy address"
                className="text-sm text-text-secondary font-mono hover:text-text-primary transition-colors"
              >
                {copied ? "Copied!" : truncateAddress(address)}
              </button>
            )}
            {authenticated ? (
              <button
                onClick={logout}
                className="px-4 py-2 text-sm rounded-lg bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors border border-border"
              >
                Disconnect
              </button>
            ) : (
              <button
                onClick={login}
                className="px-4 py-2 text-sm rounded-lg bg-accent text-white hover:bg-accent-hover transition-colors font-medium"
              >
                Connect
              </button>
            )}
          </div>
        )}

        {/* Mobile: hamburger menu */}
        {isMobile && (
          <div className="relative" ref={menuRef}>
            <button
              onClick={() => setMenuOpen(!menuOpen)}
              className="p-2 rounded-lg text-text-secondary hover:bg-bg-hover transition-colors"
              aria-label="Menu"
            >
              <svg
                className="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                {menuOpen ? (
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                ) : (
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4 6h16M4 12h16M4 18h16"
                  />
                )}
              </svg>
            </button>

            {menuOpen && (
              <div className="absolute right-0 top-full mt-1 w-56 bg-bg-secondary border border-border rounded-xl shadow-lg py-2 z-50">
                {entryCount > 0 && (
                  <div className="px-4 py-2 text-xs text-text-muted">
                    {entryCount} {entryCount === 1 ? "entry" : "entries"}
                  </div>
                )}
                {authenticated && address && (
                  <button
                    onClick={() => {
                      copyAddress();
                      setTimeout(() => setMenuOpen(false), 1000);
                    }}
                    className="w-full text-left px-4 py-2.5 text-sm text-text-secondary font-mono hover:bg-bg-hover transition-colors"
                  >
                    {copied ? "Copied!" : truncateAddress(address)}
                  </button>
                )}
                <a
                  href={FAUCET_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={() => setMenuOpen(false)}
                  className="block px-4 py-2.5 text-sm text-text-secondary hover:bg-bg-hover transition-colors"
                >
                  Faucet
                </a>
                {authenticated ? (
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      logout();
                    }}
                    className="w-full text-left px-4 py-2.5 text-sm text-danger hover:bg-bg-hover transition-colors"
                  >
                    Disconnect
                  </button>
                ) : (
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      login();
                    }}
                    className="w-full text-left px-4 py-2.5 text-sm text-accent font-medium hover:bg-bg-hover transition-colors"
                  >
                    Connect
                  </button>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  );
}
