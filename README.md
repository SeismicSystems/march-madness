# March Madness on Seismic

A private, on-chain NCAA March Madness bracket contest built on the [Seismic Network](https://seismic.systems) — where your bracket picks are hidden until the tournament starts.

## Credits

This project is heavily inspired by [jimpo's march-madness-dapp](https://github.com/jimpo/march-madness-dapp). Jim is an outrageously brilliant programmer — his ByteBracket scoring algorithm and bracket encoding scheme are used directly in our contracts. His original project was built on Ethereum with Truffle in 2018; we've ported it to Seismic's privacy-preserving network with modern tooling.

As jimpo noted in his original README, the bracket pool smart contract was also inspired by the work of the [Ethereum March Madness](https://github.com/EthereumMarchMadness) team.

## How It Works

1. **Connect** — Sign in with your Twitter, Discord, or other social account via Privy
2. **Pick** — Fill out your 64-team bracket through our web interface
3. **Submit** — Your bracket is encrypted on-chain using Seismic's shielded types. Nobody can see your picks until the deadline.
4. **Compete** — After the bracket lock deadline, all brackets are revealed. As games are played, brackets are scored using jimpo's ByteBracket algorithm.
5. **Win** — The entry (or entries) with the highest score split the prize pool equally.

### Privacy

On regular blockchains, bracket picks would be visible to everyone — giving late submitters an unfair advantage. Seismic solves this with **shielded types** (`sbytes32`): your bracket is encrypted on-chain and only revealed after the submission deadline. No commit-reveal scheme needed.

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

- **Buy-in**: 1 ETH (testnet)
- **Deadline**: Wednesday, March 18, 2026 at 12:00 PM EST
- One entry per address. You can update your bracket before the deadline.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Smart Contracts | Seismic Solidity (ssolc) + sforge |
| Client Library | TypeScript + seismic-viem |
| Frontend | React + Vite + Tailwind + seismic-react |
| Auth | Privy (social login → embedded wallet) |
| Indexer | Rust (event listener + backfill) |
| Server | Rust (HTTP, serves indexed data) |

## Project Structure

```
contracts/          — Seismic Solidity smart contracts
packages/
  client/           — TypeScript client library (bracket encoding, contract interaction)
  web/              — React web app (bracket UI, auth, live scoring)
  tests/            — Integration tests + local dev tools
crates/
  indexer/          — Rust event listener (tracks bracket submissions)
  server/           — HTTP API server
data/               — Tournament configuration (teams, regions, seeds)
docs/               — Technical docs, changeset log, prompt archive
```

## Development

### Prerequisites

- [Bun](https://bun.sh) (TypeScript runtime & package manager)
- [Rust](https://rustup.rs) (for crates)
- [sforge/sanvil](https://docs.seismic.systems/getting-started/installation) (Seismic dev tools)

### Quick Start

```bash
# Start local Seismic node
sanvil

# Build & test contracts
cd contracts && sforge test -vv

# Install TS dependencies
bun install

# Run frontend dev server
cd packages/web && bun dev

# Populate local brackets (for development)
cd packages/tests && bun run populate
```

## License

MIT
