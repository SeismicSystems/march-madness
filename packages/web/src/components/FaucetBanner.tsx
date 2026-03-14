import { useState } from "react";

import { FAUCET_URL } from "../lib/constants";
import { truncateAddress } from "../lib/tournament";

interface FaucetBannerProps {
  address: string;
}

export function FaucetBanner({ address }: FaucetBannerProps) {
  const [copied, setCopied] = useState(false);

  const copyAddress = () => {
    navigator.clipboard.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="bg-warning/10 border border-warning/30 rounded-xl p-4 sm:p-5 mb-4 sm:mb-6">
      <div className="text-sm font-semibold text-warning mb-2">
        You need ETH to submit your bracket
      </div>
      <p className="text-xs sm:text-sm text-text-secondary mb-3">
        Get free testnet ETH from the faucet. Copy your address below and paste
        it on the faucet page.
      </p>
      <div className="flex flex-col sm:flex-row gap-2 sm:items-center">
        <button
          onClick={copyAddress}
          type="button"
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-bg-tertiary border border-border text-text-primary font-mono text-xs sm:text-sm hover:bg-bg-hover transition-colors"
        >
          <span>{truncateAddress(address)}</span>
          <span className="text-text-muted">
            {copied ? "Copied!" : "[copy]"}
          </span>
        </button>
        <a
          href={FAUCET_URL}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1 px-4 py-1.5 rounded-lg bg-warning text-black font-semibold text-xs sm:text-sm hover:bg-warning/80 transition-colors"
        >
          Get Testnet ETH
        </a>
      </div>
    </div>
  );
}
