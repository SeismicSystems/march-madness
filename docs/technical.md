# Technical Architecture

## Bracket Encoding (ByteBracket)

A complete NCAA tournament bracket is encoded as a single `bytes8` value (64 bits) — identical to jimpo's original encoding:
- **Bit 63** (MSB): sentinel — MUST be 1 to distinguish submitted brackets from uninitialized storage
- **Bits 62-0**: 63 game outcomes (0 = higher seed / team1 wins, 1 = lower seed / team2 wins)

### Bit Layout

```
bytes8 (64 bits):
  Bit 63:     MSB sentinel — must be 1
  Bits 62-31: Round of 64 (32 games)
  Bits 30-15: Round of 32 (16 games)
  Bits 14-7:  Sweet 16 (8 games)
  Bits 6-3:   Elite 8 (4 games)
  Bits 2-1:   Final Four (2 games)
  Bit 0:      Championship (1 game)
```

This is jimpo's original `bytes8` encoding with one addition: we repurpose the MSB (bit 63) as a sentinel. In jimpo's version the MSB was always set to 1 as a non-zero guarantee, but wasn't explicitly validated. We require it to equal 1 on submission so we can distinguish real brackets from uninitialized mapping entries (which return all zeros). On Seismic, brackets are stored as `sbytes8` (shielded) to hide picks until the deadline.

### Team Ordering

Teams are arranged in 4 regions of 16, following the standard NCAA bracket seeding:
```
Seed order within each region: [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15]
```

Region order: East (0-15), West (16-31), South (32-47), Midwest (48-63)

Note: region order and seed ordering within regions may vary year to year — the array order in the tournament data file is what matters.

First-round matchups are adjacent pairs: team 0 vs team 1 (1-seed vs 16-seed), team 2 vs team 3 (8-seed vs 9-seed), etc.

### Final Four Pairings (2026)

- Semifinal 1: East winner vs West winner
- Semifinal 2: South winner vs Midwest winner

Note: these pairings are placeholder seed data for 2026 — the NCAA hasn't officially announced them yet. They change year to year.

## Scoring (from jimpo's ByteBracket)

```
Round     | Games | Points/correct | Max points
----------|-------|---------------|----------
Round 64  |  32   |      1        |    32
Round 32  |  16   |      2        |    32
Sweet 16  |   8   |      4        |    32
Elite 8   |   4   |      8        |    32
Final Four|   2   |     16        |    32
Champion  |   1   |     32        |    32
----------|-------|---------------|----------
Total     |  63   |               |   192
```

A pick in round N only scores if the feeder picks in round N-1 were also correct.

## Contract Lifecycle

```
1. DEPLOYMENT (owner)
   - Set submission deadline, entry fee (1 ETH), tournament data hash

2. SUBMISSION PHASE (before deadline)
   - Users submit shielded brackets (sbytes8) with 1 ETH
   - Users can update brackets (no additional fee)
   - Brackets are hidden (shielded type)
   - Events emitted: BracketSubmitted(address)

3. TOURNAMENT PHASE (after deadline, before results)
   - Submissions closed
   - All brackets now readable by anyone (getBracket access control relaxes)

4. SCORING PHASE (after owner posts results)
   - Owner calls submitResults(bytes8)
   - Anyone can call scoreBracket(address) to score individual brackets

5. PAYOUT (after all brackets scored)
   - Contract validates ALL entries scored
   - Winners (highest score) call collectWinnings()
   - Prize = total pool / number of winners

6. NO-CONTEST (28 days after results, not all scored)
   - Each entrant can call collectEntryFee() for refund
```

## Shielded Type Usage

- `sbytes8` for bracket storage: hidden until deadline
- `getBracket()` access control:
  - Before deadline: `msg.sender == account` (requires signed read via `walletClient.readContract()`)
  - After deadline: anyone can read — user client should use `.treadContract()` since data is public
- Submissions use shielded writes (`walletClient.writeContract()`), NOT transparent writes

## Client Architecture

Three access levels:
1. **Public** — no wallet, no signed reads, no writes. Always uses `.treadContract()`. Can read entry count and view brackets/results after deadline.
2. **User** — wallet connected. Can submit/update bracket (shielded write), view own bracket before deadline (signed read), view anyone's bracket after deadline (transparent read), score brackets after results posted.
3. **Owner** — same as user, plus can post results.

## Bracket Decode Formats

### JSON (default decode format)

```json
{
  "champion": {"name": "Duke", "seed": 1, "region": "East"},
  "runnerUp": {"name": "Houston", "seed": 2, "region": "Midwest"},
  "finalFour": [
    {"name": "Michigan", "seed": 1, "region": "West"},
    {"name": "Alabama", "seed": 4, "region": "South"}
  ],
  "eliteEight": [...],
  "sweetSixteen": [...],
  "roundOf32": [...],
  "games": [
    {"round": 0, "game": 0, "winner": "Duke", "loser": "Washington"},
    ...
  ]
}
```

### Human-Readable (helper function)

```
[
  "(1) Duke - Champion",
  "(2) Houston - plays in Championship",
  "(1) Michigan - Final Four",
  "(4) Alabama - Final Four",
  "(3) Purdue - Elite 8",
  ...
]
```

Ordered by achievement depth (Champion → Final Four → Elite 8 → Sweet 16 → Round of 32).

## Tournament Data File (`data/mens-2026.json`)

This single JSON file is the source of truth for all tournament configuration. It extends jimpo's original format (`name`, `teams: [{name}]`, `regions`) with additional fields for clarity:

```json
{
  "name": "NCAA Men's Basketball 2026",
  "regions": ["East", "West", "South", "Midwest"],
  "teams": [
    {"name": "Duke", "seed": 1, "region": "East", "abbrev": "DUKE"},
    ...
  ]
}
```

- **`seed`**, **`region`**, **`abbrev`** are convenience metadata for UI display and human readability.
- **The only thing that matters for on-chain encoding is the array order.** Teams are listed in jimpo's canonical order: 4 regions × 16 teams each, seeded `[1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15]` within each region. The contract and ByteBracket scoring care only about the index position of each team (0–63), not the metadata fields.

## Local Development: Populate Script

The populate script (`packages/localdev/src/populate.ts`) automates local development setup by deploying the MarchMadness contract to a sanvil node and optionally populating it with brackets, results, and scores.

### Phases

| Phase | What it does | Good for |
|-------|-------------|----------|
| `pre-submission` (default) | Deploy with future deadline (1h). No brackets. | Testing bracket picker UI, submission flow |
| `post-submission` | Deploy, submit brackets from 10 anvil accounts, fast-forward past deadline, post results (chalky bracket), score first 3 brackets | Testing bracket viewing, scoring UI, leaderboard. Remaining brackets left unscored for manual testing |
| `post-grading` | Full lifecycle: deploy, submit, score ALL brackets, fast-forward past 7-day scoring window | Testing payout collection, final leaderboard, `collectWinnings()` |

### Usage

```bash
bun run --filter @march-madness/localdev populate                              # pre-submission (default)
bun run --filter @march-madness/localdev populate -- --phase post-submission    # brackets + results
bun run --filter @march-madness/localdev populate -- --phase post-grading      # full lifecycle
bun run --filter @march-madness/localdev populate -- --rpc-url http://host:port
```

### Environment Variables

All env vars live in a single `.env` file at the repo root (see `.env.example`).

**Populate script (local dev):**

| Variable | Description | Default |
|----------|------------|---------|
| `CONTRACT_ADDRESS` | Use existing contract (skip deploy) | — |
| `RPC_URL` | RPC endpoint (overridden by `--rpc-url`) | `http://localhost:8545` |
| `DEADLINE_OFFSET` | Deadline offset in seconds | `3600` (pre-submission) |

**Deployment + Frontend (VITE_ vars are shared):**

| Variable | Description | Default |
|----------|------------|---------|
| `DEPLOYER_PRIVATE_KEY` | Private key for signing deploy tx | — (required for testnet) |
| `VITE_PRIVY_APP_ID` | Privy app ID | `"placeholder-app-id"` |
| `VITE_CONTRACT_ADDRESS` | MarchMadness contract address | zero address |
| `VITE_CHAIN_ID` | Chain ID for wallet config | sanvil chain ID |
| `VITE_RPC_URL` | RPC URL — used by both frontend and `bun deploy:testnet` | — |
| `VITE_PUBLIC_RPC_URL` | Public RPC for transparent reads | — |

### Test Accounts

The script uses sanvil's pre-funded accounts from `data/anvil-accounts.json`. Account 0 is the deployer/owner; accounts 1-10 are players. In `post-submission` phase, player 3 (index 2) always gets the "chalky" bracket (all higher seeds win) which matches the results — guaranteeing a perfect score of 192.

## Integration Tests

The integration test suite (`packages/localdev/test/integration.test.ts`) validates the full contract lifecycle against a live sanvil node using the `@march-madness/client` library:

1. Contract deployment (via sforge or direct)
2. Bracket submission and update
3. Deadline enforcement (fast-forward via `evm_increaseTime`)
4. Results posting and bracket scoring
5. Winner determination and payout collection

```bash
bun run --filter @march-madness/localdev test
```

Tests require sanvil running on `localhost:8545`.
