import { useRef, useState, useEffect } from "react";
import { Link, useLocation } from "react-router-dom";
import { usePrivy } from "@privy-io/react-auth";

import { FAUCET_URL } from "../lib/constants";
import { truncateAddress } from "../lib/tournament";
import { useIsMobile } from "../hooks/useIsMobile";
import { useStats } from "../hooks/useStats";

export function Header() {
  const { login, logout, authenticated, user } = usePrivy();
  const [copied, setCopied] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const isMobile = useIsMobile();
  const { totalEntries } = useStats();
  const location = useLocation();

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

  const navLinkClass = (path: string) => {
    const active = location.pathname === path;
    return `px-3 py-2 text-sm rounded-lg transition-colors ${
      active
        ? "bg-bg-tertiary text-text-primary font-medium"
        : "text-text-secondary hover:text-text-primary hover:bg-bg-hover"
    }`;
  };

  return (
    <header className="border-b border-border bg-bg-secondary/80 backdrop-blur-sm sticky top-0 z-50">
      <div className="max-w-[1800px] mx-auto px-3 sm:px-4 py-2 sm:py-3 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 sm:gap-4 min-w-0">
          <Link to="/" className="flex items-center gap-1 sm:gap-2">
            <h1 className="text-base sm:text-xl font-bold text-text-primary tracking-tight whitespace-nowrap flex items-center">
              March Madness
              <img
                src="/seis_logo.png"
                alt="Seismic"
                className="h-4 sm:h-5 ml-1.5 sm:ml-2 inline-block"
              />
            </h1>
          </Link>
          {!isMobile && (
            <nav className="flex items-center gap-1 ml-2">
              <Link to="/" className={navLinkClass("/")}>
                Bracket
              </Link>
              <Link to="/groups" className={navLinkClass("/groups")}>
                Groups
              </Link>
              <Link to="/leaderboard" className={navLinkClass("/leaderboard")}>
                Leaderboard
              </Link>
            </nav>
          )}
        </div>

        {/* Desktop: inline buttons */}
        {!isMobile && (
          <div className="flex items-center gap-3 shrink-0">
            {totalEntries != null && (
              <div className="flex flex-col items-center leading-none px-2">
                <span className="text-lg font-bold text-text-primary">{totalEntries}</span>
                <span className="text-[10px] text-text-secondary">Brackets</span>
              </div>
            )}
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
                className="cursor-pointer text-sm text-text-secondary font-mono hover:text-text-primary hover:underline decoration-dotted underline-offset-4 transition-colors"
              >
                {copied ? "Copied!" : truncateAddress(address)}
              </button>
            )}
            {authenticated ? (
              <button
                onClick={logout}
                className="cursor-pointer px-4 py-2 text-sm rounded-lg bg-bg-tertiary text-text-secondary hover:bg-bg-hover hover:text-text-primary transition-colors border border-border"
              >
                Disconnect
              </button>
            ) : (
              <button
                onClick={login}
                className="cursor-pointer px-4 py-2 text-sm rounded-lg bg-accent text-white hover:bg-accent-hover transition-colors font-medium"
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
                <Link
                  to="/"
                  onClick={() => setMenuOpen(false)}
                  className="block px-4 py-2.5 text-sm text-text-secondary hover:bg-bg-hover transition-colors"
                >
                  Bracket
                </Link>
                <Link
                  to="/groups"
                  onClick={() => setMenuOpen(false)}
                  className="block px-4 py-2.5 text-sm text-text-secondary hover:bg-bg-hover transition-colors"
                >
                  Groups
                </Link>
                <Link
                  to="/leaderboard"
                  onClick={() => setMenuOpen(false)}
                  className="block px-4 py-2.5 text-sm text-text-secondary hover:bg-bg-hover transition-colors"
                >
                  Leaderboard
                </Link>
                {authenticated && address && (
                  <button
                    onClick={() => {
                      copyAddress();
                      setTimeout(() => setMenuOpen(false), 1000);
                    }}
                    className="w-full cursor-pointer text-left px-4 py-2.5 text-sm text-text-secondary font-mono hover:bg-bg-hover transition-colors"
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
                    className="w-full cursor-pointer text-left px-4 py-2.5 text-sm text-danger hover:bg-bg-hover transition-colors"
                  >
                    Disconnect
                  </button>
                ) : (
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      login();
                    }}
                    className="w-full cursor-pointer text-left px-4 py-2.5 text-sm text-accent font-medium hover:bg-bg-hover transition-colors"
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
