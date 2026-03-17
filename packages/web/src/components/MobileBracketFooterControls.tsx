import { useRef, useState } from "react";

interface MobileBracketFooterControlsProps {
  encodedBracket: `0x${string}` | null;
  isLocked: boolean;
  onLoadHex: (hex: `0x${string}`) => void;
}

export function MobileBracketFooterControls({
  encodedBracket,
  isLocked,
  onLoadHex,
}: MobileBracketFooterControlsProps) {
  const [hexSectionOpen, setHexSectionOpen] = useState(false);
  const [hexOpen, setHexOpen] = useState(false);
  const [hexInput, setHexInput] = useState("");
  const [hexError, setHexError] = useState<string | null>(null);
  const [hexCopied, setHexCopied] = useState(false);
  const hexRef = useRef<HTMLInputElement>(null);

  const handleCopy = async () => {
    if (!encodedBracket) return;
    await navigator.clipboard.writeText(encodedBracket);
    setHexCopied(true);
    setTimeout(() => setHexCopied(false), 1200);
  };

  const tryLoadHex = (raw: string) => {
    const cleaned = raw.replace(/[^0-9a-fA-Fx]/g, "");
    if (!/^0x[0-9a-fA-F]{16}$/.test(cleaned)) {
      setHexError(null);
      return false;
    }
    const firstNibble = parseInt(cleaned[2], 16);
    if (firstNibble < 8) {
      const fixedNibble = (firstNibble | 0x8).toString(16);
      const fixed = `0x${fixedNibble}${cleaned.slice(3)}` as `0x${string}`;
      setHexError(
        `Pasted ${cleaned} — missing sentinel bit. Loaded ${fixed} (same picks, valid for submission).`,
      );
      onLoadHex(fixed);
    } else {
      setHexError(null);
      onLoadHex(cleaned as `0x${string}`);
    }
    setHexInput("");
    setHexOpen(false);
    return true;
  };

  const handleHexChange = (value: string) => {
    setHexInput(value);
    tryLoadHex(value);
  };

  const handleHexPaste = (e: React.ClipboardEvent) => {
    const pasted = e.clipboardData.getData("text");
    if (tryLoadHex(pasted)) {
      e.preventDefault();
    }
  };

  return (
    <div className="mt-4 space-y-3">
      <button
        onClick={() => {
          const next = !hexSectionOpen;
          setHexSectionOpen(next);
          if (!next) {
            setHexOpen(false);
            setHexInput("");
          }
        }}
        className="text-[10px] font-mono text-text-muted/30 hover:text-text-muted/55 transition-colors"
      >
        {hexSectionOpen ? "hide hex" : "0x"}
      </button>

      {hexSectionOpen && (
        <div className="space-y-2">
          <div className="text-[10px] font-mono text-text-muted/45 break-all select-all">
            {encodedBracket ?? "0x..."}
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={handleCopy}
              disabled={!encodedBracket}
              className="text-[10px] font-mono text-text-muted/45 hover:text-text-muted disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              {hexCopied ? "copied" : "copy"}
            </button>
            {!isLocked && (
              <button
                onClick={() => {
                  setHexOpen((prev) => !prev);
                  setTimeout(() => hexRef.current?.focus(), 0);
                }}
                className="text-[10px] font-mono text-text-muted/45 hover:text-text-muted transition-colors"
              >
                {hexOpen ? "close" : "enter hex"}
              </button>
            )}
          </div>

          {!isLocked && hexOpen && (
            <input
              ref={hexRef}
              type="text"
              value={hexInput}
              onChange={(e) => handleHexChange(e.target.value)}
              onPaste={handleHexPaste}
              onBlur={() => {
                if (!hexRef.current?.value) setHexOpen(false);
              }}
              placeholder="paste 0x..."
              spellCheck={false}
              autoFocus
              className="w-full py-1 text-[10px] font-mono bg-transparent border-0 border-b border-border/40 text-text-muted focus:outline-none focus:border-accent/50 transition-colors"
            />
          )}

          {hexError && (
            <div className="text-[10px] text-warning/80">{hexError}</div>
          )}
        </div>
      )}
    </div>
  );
}
