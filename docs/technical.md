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

This is jimpo's original `bytes8` encoding unchanged. On Seismic, brackets are stored as `sbytes8` (shielded) to hide picks until the deadline.

### Team Ordering

Teams are arranged in 4 regions of 16, following the standard NCAA bracket seeding:
```
Seed order within each region: [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15]
```

Region order: East (0-15), West (16-31), South (32-47), Midwest (48-63)

First-round matchups are adjacent pairs: team 0 vs team 1 (1-seed vs 16-seed), team 2 vs team 3 (8-seed vs 9-seed), etc.

### Final Four Pairings (2026)

- Semifinal 1: East winner vs West winner
- Semifinal 2: South winner vs Midwest winner

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
  - After deadline: anyone can read (client should use `.treadContract()` since data is public)
- Submissions use shielded writes (`walletClient.writeContract()`), NOT transparent writes

## Client Architecture

Three access levels:
1. **Public** — no wallet needed: read entry count, view results after deadline
2. **User** — wallet connected: submit/update bracket, view own bracket (signed read), score brackets
3. **Owner** — special wallet: post results, all user capabilities

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
