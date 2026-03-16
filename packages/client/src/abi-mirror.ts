// BracketMirror contract ABI — derived from contracts/src/BracketMirror.sol
//
// DO NOT EDIT MANUALLY. Rebuild contracts with sforge and copy the abi field.

export const BracketMirrorAbi = [
  // ── View functions ──────────────────────────────────────────
  {
    type: "function",
    name: "nextMirrorId",
    inputs: [],
    outputs: [
      { name: "", type: "uint256", internalType: "uint256" },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "MAX_SLUG_LENGTH",
    inputs: [],
    outputs: [
      { name: "", type: "uint256", internalType: "uint256" },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "slugToMirrorId",
    inputs: [
      { name: "", type: "bytes32", internalType: "bytes32" },
    ],
    outputs: [
      { name: "", type: "uint256", internalType: "uint256" },
    ],
    stateMutability: "view",
  },
  // ── Mirror lifecycle ────────────────────────────────────────
  {
    type: "function",
    name: "createMirror",
    inputs: [
      { name: "slug", type: "string", internalType: "string" },
      { name: "displayName", type: "string", internalType: "string" },
    ],
    outputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
    ],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "setEntryFee",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "fee", type: "uint32", internalType: "uint32" },
      { name: "currency", type: "string", internalType: "string" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  // ── Entry management (admin only) ──────────────────────────
  {
    type: "function",
    name: "addEntry",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "bracket", type: "bytes8", internalType: "bytes8" },
      { name: "slug", type: "string", internalType: "string" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "removeEntry",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "entryIndex", type: "uint256", internalType: "uint256" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "updateBracket",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "entryIndex", type: "uint256", internalType: "uint256" },
      { name: "bracket", type: "bytes8", internalType: "bytes8" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "updateEntrySlug",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "entryIndex", type: "uint256", internalType: "uint256" },
      { name: "slug", type: "string", internalType: "string" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  // ── Read helpers ────────────────────────────────────────────
  {
    type: "function",
    name: "getMirrorBySlug",
    inputs: [
      { name: "slug", type: "string", internalType: "string" },
    ],
    outputs: [
      { name: "", type: "uint256", internalType: "uint256" },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getMirror",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
    ],
    outputs: [
      {
        name: "",
        type: "tuple",
        internalType: "struct BracketMirror.Mirror",
        components: [
          { name: "slug", type: "string", internalType: "string" },
          { name: "displayName", type: "string", internalType: "string" },
          { name: "entryFee", type: "uint32", internalType: "uint32" },
          { name: "entryCurrency", type: "string", internalType: "string" },
          { name: "admin", type: "address", internalType: "address" },
        ],
      },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getEntryCount",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
    ],
    outputs: [
      { name: "", type: "uint256", internalType: "uint256" },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getEntry",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "index", type: "uint256", internalType: "uint256" },
    ],
    outputs: [
      {
        name: "",
        type: "tuple",
        internalType: "struct BracketMirror.MirrorEntry",
        components: [
          { name: "bracket", type: "bytes8", internalType: "bytes8" },
          { name: "slug", type: "string", internalType: "string" },
        ],
      },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getEntryBySlug",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
      { name: "slug", type: "string", internalType: "string" },
    ],
    outputs: [
      {
        name: "",
        type: "tuple",
        internalType: "struct BracketMirror.MirrorEntry",
        components: [
          { name: "bracket", type: "bytes8", internalType: "bytes8" },
          { name: "slug", type: "string", internalType: "string" },
        ],
      },
    ],
    stateMutability: "view",
  },
  {
    type: "function",
    name: "getEntries",
    inputs: [
      { name: "mirrorId", type: "uint256", internalType: "uint256" },
    ],
    outputs: [
      {
        name: "",
        type: "tuple[]",
        internalType: "struct BracketMirror.MirrorEntry[]",
        components: [
          { name: "bracket", type: "bytes8", internalType: "bytes8" },
          { name: "slug", type: "string", internalType: "string" },
        ],
      },
    ],
    stateMutability: "view",
  },
  // ── Events ──────────────────────────────────────────────────
  {
    type: "event",
    name: "MirrorCreated",
    inputs: [
      { name: "mirrorId", type: "uint256", indexed: true, internalType: "uint256" },
      { name: "slug", type: "string", indexed: false, internalType: "string" },
      { name: "displayName", type: "string", indexed: false, internalType: "string" },
      { name: "admin", type: "address", indexed: false, internalType: "address" },
    ],
    anonymous: false,
  },
  {
    type: "event",
    name: "EntryAdded",
    inputs: [
      { name: "mirrorId", type: "uint256", indexed: true, internalType: "uint256" },
      { name: "entryIndex", type: "uint256", indexed: false, internalType: "uint256" },
      { name: "slug", type: "string", indexed: false, internalType: "string" },
    ],
    anonymous: false,
  },
  {
    type: "event",
    name: "EntryRemoved",
    inputs: [
      { name: "mirrorId", type: "uint256", indexed: true, internalType: "uint256" },
      { name: "entryIndex", type: "uint256", indexed: false, internalType: "uint256" },
    ],
    anonymous: false,
  },
  {
    type: "event",
    name: "BracketUpdated",
    inputs: [
      { name: "mirrorId", type: "uint256", indexed: true, internalType: "uint256" },
      { name: "entryIndex", type: "uint256", indexed: false, internalType: "uint256" },
    ],
    anonymous: false,
  },
  // ── Errors ──────────────────────────────────────────────────
  { type: "error", name: "MirrorDoesNotExist", inputs: [] },
  { type: "error", name: "NotMirrorAdmin", inputs: [] },
  { type: "error", name: "SlugCannotBeEmpty", inputs: [] },
  { type: "error", name: "SlugTooLong", inputs: [] },
  { type: "error", name: "SlugAlreadyTaken", inputs: [] },
  { type: "error", name: "InvalidSentinelByte", inputs: [] },
  { type: "error", name: "EntrySlugAlreadyTaken", inputs: [] },
  { type: "error", name: "IndexOutOfBounds", inputs: [] },
  { type: "error", name: "MirrorNotFound", inputs: [] },
  { type: "error", name: "EntryNotFound", inputs: [] },
] as const;
