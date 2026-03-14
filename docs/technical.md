# Technical Architecture

## Bracket Encoding (ByteBracket)

A complete NCAA tournament bracket is encoded as a single `bytes32` value:
- **63 bits** encode 63 game outcomes (0 = higher seed / team1 wins, 1 = lower seed / team2 wins)
- **1 sentinel byte**: the last byte (byte index 31) MUST equal `0x01` to distinguish submitted brackets from uninitialized storage

### Bit Layout

```
Byte 0-7 (bits 63-0):   Game outcomes
  Bits 62-31: Round of 64 (32 games)
  Bits 30-15: Round of 32 (16 games)
  Bits 14-7:  Sweet 16 (8 games)
  Bits 6-3:   Elite 8 (4 games)
  Bits 2-1:   Final Four (2 games)
  Bit 0:      Championship (1 game)
  Bit 63:     MSB — set to 1 (non-zero guarantee in jimpo's scheme)

Bytes 8-30:  Unused (zero)
Byte 31:     Sentinel = 0x01
```

**Note**: We use `bytes32` instead of jimpo's `bytes8` because Seismic's `sbytes32` is the natural shielded type. The game bits occupy the first 8 bytes; bytes 8-30 are zero; byte 31 is the sentinel.

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
   - Users submit shielded brackets (sbytes32) with 1 ETH
   - Users can update brackets (no additional fee)
   - Brackets are hidden (shielded type)
   - Events emitted: BracketSubmitted(address)

3. TOURNAMENT PHASE (after deadline, before results)
   - Submissions closed
   - All brackets now readable by anyone (getBracket access control relaxes)

4. SCORING PHASE (after owner posts results)
   - Owner calls submitResults(bytes32)
   - Anyone can call scoreBracket(address) to score individual brackets

5. PAYOUT (after all brackets scored)
   - Contract validates ALL entries scored
   - Winners (highest score) call collectWinnings()
   - Prize = total pool / number of winners

6. NO-CONTEST (28 days after results, not all scored)
   - Each entrant can call collectEntryFee() for refund
```

## Shielded Type Usage

- `sbytes32` for bracket storage: hidden until deadline
- `getBracket()` access control:
  - Before deadline: `msg.sender == account` (requires signed read)
  - After deadline: anyone can read (returns `uint256(bracket)` for public access)
- Submissions use shielded writes (seismic transaction, NOT transparent)

## Client Architecture

Three access levels:
1. **Public** — no wallet needed: read entry count, view results after deadline
2. **User** — wallet connected: submit/update bracket, view own bracket (signed read), score brackets
3. **Owner** — special wallet: post results, all user capabilities

## Human-Readable Bracket Format

```json
[
  "(1) Duke - Champion",
  "(2) Houston - plays in Championship",
  "(1) Michigan - Final Four",
  "(4) Alabama - Final Four",
  "(3) Purdue - Elite 8",
  "(2) Illinois - Elite 8",
  ...
]
```

Ordered by achievement depth (Champion → Final Four → Elite 8 → Sweet 16 → Round of 32).
