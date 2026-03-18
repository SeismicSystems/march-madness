# March Madness on Seismic

A private, on-chain NCAA March Madness bracket contest built on the [Seismic Network](https://seismic.systems) — where your bracket picks are hidden until the tournament starts.

Hosted at [brackets.seismictest.net](https://brackets.seismictest.net)

## Credits

Built on [jimpo's march-madness-dapp](https://github.com/jimpo/march-madness-dapp). The bracket encoding and scoring use his ByteBracket library directly, which implements the compact scoring algorithm by [pursuingpareto](https://gist.github.com/pursuingpareto/b15f1197d96b1a2bbc48). jimpo's original project ran on Ethereum with Truffle; we've ported it to Seismic with modern tooling.

Aside from ripping a bunch of contracts from Jim: this whole thing was super obviously vibe coded.

## How It Works

1. **Connect** — Sign in with your Twitter, Discord, or other social account via Privy
2. **Pick** — Fill out your 64-team bracket through our web interface
3. **Submit** — Your bracket is encrypted on-chain using Seismic's shielded types. Nobody can see your picks until the deadline.
4. **Compete** — After the bracket lock deadline, all brackets are revealed. As games are played, brackets are scored using jimpo's ByteBracket algorithm.
5. **Win** — The entry (or entries) with the highest score split the prize pool equally.

### Privacy

On regular blockchains, bracket picks would be visible to everyone — giving late submitters an unfair advantage. Seismic solves this with **shielded types** (`sbytes8`): your bracket is encrypted on-chain and only revealed after the submission deadline. No commit-reveal scheme needed.

### Scoring

Scoring follows jimpo's ByteBracket algorithm:
- **Round of 64**: 1 point per correct pick (32 games)
- **Round of 32**: 2 points per correct pick (16 games)
- **Sweet 16**: 4 points per correct pick (8 games)
- **Elite 8**: 8 points per correct pick (4 games)
- **Final Four**: 16 points per correct pick (2 games)
- **Championship**: 32 points (1 game)
- **Maximum possible score**: 192 points

A later-round pick only scores if the feeder games were also picked correctly.

### Entry

- **Buy-in**: 0.1 ETH (testnet)
- **Deadline**: Thursday, March 19, 2026 at 12:15 PM EST
- One entry per address. You can update your bracket before the deadline.

### Mirrors & Groups

Two separate contracts for side pools alongside the main contest:

- **Mirrors** (`BracketMirror`): Admin enters external brackets (bracket + slug) from off-chain pools (e.g. Yahoo Fantasy). No money, no scoring on-chain — purely for display. Admin sets a prize description for bookkeeping.
- **Groups** (`BracketGroups`): Creating a group auto-joins the creator (default name "CREATOR", editable). Other users self-join with their main-contract bracket. Optional password protection (`sbytes12(keccak256("your-password"))`). Optional entry fee creates a side-bet prize pool. Winners split the pool after scoring.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Smart Contracts | Seismic Solidity (ssolc) + sforge |
| Client Library | TypeScript + seismic-viem |
| Frontend | React + Vite + Tailwind + seismic-react |
| Auth | Privy (social login → embedded wallet) |
| Indexer | Rust (event listener + backfill) |
| Server | Rust (HTTP, serves indexed data + tournament status + forecasts) |
| Forecaster | Rust (Monte Carlo bracket win probability simulator) |
| Calibrator | Rust (goose fitting via market-making loss against Kalshi orderbooks) |
| NCAA Feed | Rust (live score polling → Redis) |

## Pages

| Route | Description |
|-------|-------------|
| `/` | Bracket picker (pre-deadline) or own bracket with tournament overlay (post-deadline) |
| `/leaderboard` | All entries ranked by score with current/max points, champion pick |
| `/groups` | Create and join bracket groups (public or private with passphrase) |
| `/bracket/:address` | Read-only bracket view for any player with tournament status overlay |

## Mirrors & Groups

In addition to the main bracket contest, the platform supports two types of sub-pools:

- **Mirrors** (`BracketMirror.sol`) — Admin-managed pools that mirror external bracket contests (e.g. Yahoo Fantasy). No money, no on-chain scoring. The admin enters brackets + slugs manually; all winner computation is off-chain. Entry slugs are unique within a mirror for URL-friendly lookup.
- **Groups** (`BracketGroups.sol`) — Linked sub-groups where members join with their main-contract bracket. Optional password protection (`sbytes12`, shielded) and entry fee with winner payout. Scoring delegates to the main contract to avoid double work.

Both contracts are deployed alongside MarchMadness via a unified deploy script. BracketGroups composes with MarchMadness through a minimal `IMarchMadness` interface.

## Project Structure

```
contracts/          — Seismic Solidity smart contracts (MarchMadness, BracketGroups, BracketMirror)
packages/
  client/           — TypeScript client library (bracket encoding, scoring, contract interaction)
  web/              — React web app (bracket UI, auth, leaderboard, bracket viewer)
  localdev/         — Local dev tools + integration tests
crates/
  seismic-march-madness/ — Shared library: types, scoring, simulation, tournament helpers
  kalshi/           — Kalshi odds ingestor (REST + WS + orderbook fetching)
  bracket-sim/      — Tournament simulation, calibration (CSV + market-making modes)
  indexer/          — Rust event listener (tracks bracket submissions)
  server/           — HTTP API server (entries + tournament status + forecasts)
  forecaster/       — Monte Carlo win probability simulator
  ncaa-api/         — NCAA basketball API client (scoreboard + schedule + bracket)
  ncaa-feed/        — NCAA live score feed + bracket fetcher (fetch-bracket binary)
data/               — Tournament data, seed configs
docs/               — Technical docs, changeset log, prompt archive
```

## Development

### Prerequisites

- [Bun](https://bun.sh) (TypeScript runtime & package manager)
- [Rust](https://rustup.rs) (for crates)
- [sforge/sanvil](https://docs.seismic.systems/getting-started/installation) (Seismic dev tools)

### Quick Start

```bash
# Copy env template and fill in values
cp .env.example .env

# Build & test contracts
cd contracts && sforge test -vv

# Install TS dependencies
bun install

# Run frontend dev server
cd packages/web && bun dev

# Populate local state for development (spawns sanvil, deploys via sforge, populates brackets)
bun p:pre                     # pre-submission (default): deploy with future deadline, no brackets
bun p:post                    # post-submission: brackets + results + partial scoring
bun p:grading                 # post-grading: full lifecycle including payouts

# Deploy to testnet — deploys + writes address to deployments.ts
bun deploy:testnet

# Or just write an already-deployed address
./scripts/deploy-testnet.sh --contract-address 0x1234...
```

### Environment

All environment variables live in a single `.env` file at the repo root. See `.env.example` for the full list. Vite loads from there via `envDir`, and the testnet deploy script sources it directly. The local populate script uses hardcoded anvil accounts — it does not need `DEPLOYER_PRIVATE_KEY`.

## Deployment

Production deployment docs, nginx config, and supervisor config live in `deploy/`. See [`deploy/README.md`](deploy/README.md) for full setup instructions.

## License

MIT
