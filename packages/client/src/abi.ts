// MarchMadness contract ABI.
// For shielded types (sbytes8), we use bytes8 in the ABI —
// seismic-viem handles the shielding transparently.

export const MarchMadnessAbi = [
  // ── Constructor ──
  {
    type: "constructor",
    inputs: [
      { name: "_entryFee", type: "uint256" },
      { name: "_submissionDeadline", type: "uint256" },
    ],
    stateMutability: "nonpayable",
  },

  // ── Bracket Submission ──
  {
    type: "function",
    name: "submitBracket",
    inputs: [{ name: "bracket", type: "bytes8" }],
    outputs: [],
    stateMutability: "payable",
  },
  {
    type: "function",
    name: "updateBracket",
    inputs: [{ name: "bracket", type: "bytes8" }],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "setTag",
    inputs: [{ name: "tag", type: "string" }],
    outputs: [],
    stateMutability: "nonpayable",
  },

  // ── Bracket Reading ──
  {
    type: "function",
    name: "getBracket",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ name: "", type: "bytes8" }],
    stateMutability: "view",
  },

  // ── Results ──
  {
    type: "function",
    name: "submitResults",
    inputs: [{ name: "_results", type: "bytes8" }],
    outputs: [],
    stateMutability: "nonpayable",
  },

  // ── Scoring ──
  {
    type: "function",
    name: "scoreBracket",
    inputs: [{ name: "account", type: "address" }],
    outputs: [],
    stateMutability: "nonpayable",
  },

  // ── Payout ──
  {
    type: "function",
    name: "collectWinnings",
    inputs: [],
    outputs: [],
    stateMutability: "nonpayable",
  },

  // ── View Functions ──
  {
    type: "function",
    name: "getEntryCount",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getScore",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ name: "", type: "uint8" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getIsScored",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getTag",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ name: "", type: "string" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "numEntries",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "submissionDeadline",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "results",
    inputs: [],
    outputs: [{ name: "", type: "bytes8" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "owner",
    inputs: [],
    outputs: [{ name: "", type: "address" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "entryFee",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "scores",
    inputs: [{ name: "", type: "address" }],
    outputs: [{ name: "", type: "uint8" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "isScored",
    inputs: [{ name: "", type: "address" }],
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "tags",
    inputs: [{ name: "", type: "address" }],
    outputs: [{ name: "", type: "string" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "numScored",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "scoringMask",
    inputs: [],
    outputs: [{ name: "", type: "uint64" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "resultsPostedAt",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "winningScore",
    inputs: [],
    outputs: [{ name: "", type: "uint8" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "numWinners",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "hasCollectedWinnings",
    inputs: [{ name: "", type: "address" }],
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "SCORING_DURATION",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  },

  // ── Events ──
  {
    type: "event",
    name: "BracketSubmitted",
    inputs: [{ name: "account", type: "address", indexed: true }],
  },
  {
    type: "event",
    name: "TagSet",
    inputs: [
      { name: "account", type: "address", indexed: true },
      { name: "tag", type: "string", indexed: false },
    ],
  },
  {
    type: "event",
    name: "BracketScored",
    inputs: [
      { name: "account", type: "address", indexed: true },
      { name: "score", type: "uint8", indexed: false },
    ],
  },
  {
    type: "event",
    name: "ResultsPosted",
    inputs: [{ name: "results", type: "bytes8", indexed: false }],
  },
  {
    type: "event",
    name: "WinningsCollected",
    inputs: [
      { name: "account", type: "address", indexed: true },
      { name: "amount", type: "uint256", indexed: false },
    ],
  },
] as const;
