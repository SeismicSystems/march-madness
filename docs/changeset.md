# Changeset

All notable changes to this project. Every PR must add an entry here.

fix(ncaa-feed): auto-detect date rollover for multi-day tournament polling

The feed previously determined the contest date once at startup and never updated it.
During a multi-day tournament, this meant the feed would keep polling a stale date's
scoreboard and miss all games on subsequent days. Now re-detects the current game day
from the NCAA schedule API whenever the feed is in PreGame phase (no live games),
which naturally covers overnight rollovers and between-day gaps.


feat(web): desktop FF sort controls inline with header row


feat(web): move sort controls to top-right on desktop FF view; rename "Final Four Picks" to "Final Four"


feat(web): FF mirror desktop — champion groups as vertical columns with sort/reorder


Desktop Final Four view groups entries by champion pick. Champions with 2+ entries get their own named section; remaining entries are pooled by seed into "Other N-seeds" or "Other seeds" groups. Each section uses the same mobile card layout in a 2-3 column grid. Sorting via the footer applies within each group.


Fix FF sort footer alignment: left-align sort options, right-align "Saved order" button so sort controls don't shift when the button appears/disappears


Add champion filter to leaderboard page with typeahead search


Final Four page: add sort footer (Win %, E[pts], Score) with "↩ Saved order" to revert to custom ordering. Replace mobile card tap with explicit "Bracket →" button. Remove "Champion" label. Show per-entry win%, expected score, and current/max in top bar. Fix champion chip centering via 3-col grid.


Clean up Final Four comparison layout: remove "Semifinal 1/2" headers, "vs" dividers, and arrows. Show F4 teams in tight 2x2 grid, finalists below, champion unchanged. Add per-entry up/down reorder buttons with localStorage persistence keyed by mirror slug.

Add Final Four comparison page for mirror pools (`/mirrors/id/:id/ff`). Shows all entries' F4 picks, semifinal winners, and champion in a bracket-flow layout with team advance probabilities and eliminated/advancing overlays. Separate mobile (cards) and desktop (horizontal bracket rows) views. Reusable `FinalFourComparison` component with generic interface for future use with groups and leaderboard.


Clean up Final Four comparison layout: remove "Semifinal 1/2" headers, "vs" dividers, and arrows. Show F4 teams in tight 2x2 grid, finalists below, champion unchanged. Add per-entry up/down reorder buttons with localStorage persistence keyed by mirror slug.

Add Final Four comparison page for mirror pools (`/mirrors/id/:id/ff`). Shows all entries' F4 picks, semifinal winners, and champion in a bracket-flow layout with team advance probabilities and eliminated/advancing overlays. Separate mobile (cards) and desktop (horizontal bracket rows) views. Reusable `FinalFourComparison` component with generic interface for future use with groups and leaderboard.


Clean up Final Four comparison layout: remove "Semifinal 1/2" headers, "vs" dividers, and arrows. Show F4 teams in tight 2x2 grid, finalists below, champion unchanged. Add per-entry up/down reorder buttons with localStorage persistence keyed by mirror slug.

Add Final Four comparison page for mirror pools (`/mirrors/id/:id/ff`). Shows all entries' F4 picks, semifinal winners, and champion in a bracket-flow layout with team advance probabilities and eliminated/advancing overlays. Separate mobile (cards) and desktop (horizontal bracket rows) views. Reusable `FinalFourComparison` component with generic interface for future use with groups and leaderboard.


Clean up Final Four comparison layout: remove "Semifinal 1/2" headers, "vs" dividers, and arrows. Show F4 teams in tight 2x2 grid, finalists below, champion unchanged. Add per-entry up/down reorder buttons with localStorage persistence keyed by mirror slug.

Add Final Four comparison page for mirror pools (`/mirrors/id/:id/ff`). Shows all entries' F4 picks, semifinal winners, and champion in a bracket-flow layout with team advance probabilities and eliminated/advancing overlays. Separate mobile (cards) and desktop (horizontal bracket rows) views. Reusable `FinalFourComparison` component with generic interface for future use with groups and leaderboard.


Clean up Final Four comparison layout: remove "Semifinal 1/2" headers, "vs" dividers, and arrows. Show F4 teams in tight 2x2 grid, finalists below, champion unchanged. Add per-entry up/down reorder buttons with localStorage persistence keyed by mirror slug.

Add Final Four comparison page for mirror pools (`/mirrors/id/:id/ff`). Shows all entries' F4 picks, semifinal winners, and champion in a bracket-flow layout with team advance probabilities and eliminated/advancing overlays. Separate mobile (cards) and desktop (horizontal bracket rows) views. Reusable `FinalFourComparison` component with generic interface for future use with groups and leaderboard.


Add Final Four comparison page for mirror pools (`/mirrors/id/:id/ff`). Shows all entries' F4 picks, semifinal winners, and champion in a bracket-flow layout with team advance probabilities and eliminated/advancing overlays. Separate mobile (cards) and desktop (horizontal bracket rows) views. Reusable `FinalFourComparison` component with generic interface for future use with groups and leaderboard.


Fix Rust live forecaster scoring to use the correct ByteBracket bit ordering


ncaa-feed: shift date detection by -3h for late games and add quiet hours (1am–12pm ET) to skip API calls when no games are live


Fix ncaa-feed date auto-detection to use America/New_York timezone instead of UTC, and add 5 missing NCAA name mappings (LIU, Miami (OH), Queens (NC), Prairie View, California Baptist).


Add configurable `-u`/`--kenpom-update-factor` flags to the tournament simulation CLIs so Bayesian postgame metric updates can be tuned without rebuilding.


Fix halftime games showing "LIVE" instead of "HALF" in live game banner. At halftime, the NCAA API sends an empty clock string with `currentPeriod: "HALF"` — normalize `clock_seconds` to `Some(0)` so the frontend correctly displays "HALF".


Split tied forecaster pool leaders fractionally so live forecast win probabilities match oddsmaker.


Add mobile Live tab that shows currently in-progress games in a single-column grid, defaulting to it when live games exist


Tighten bracket slot spacing: reduce seed width, move logos closer to seeds, reduce gap between live probability and score


Extract shared LeaderboardTable component from LeaderboardPage and MirrorLeaderboardPage, eliminating ~300 lines of duplicated sort/pagination/table rendering logic. Both pages are now thin data-fetching wrappers. Also tighten column spacing on wide screens so numeric columns stay compact and the player name column absorbs extra space.


Stabilize leaderboard sort order across data refreshes. Rows no longer reshuffle every 30s when scores/forecasts update — values update in place and order only changes on explicit user sort actions. Also updates default score sort tiebreaker chain to: current score → P(win) → E[score].


Fix forecaster expected scores by using full tournament simulation (simulate_tournament_bb with Bayesian metric updates) when all games are upcoming, instead of the forward sim which uses static metrics. This matches the oddsmaker's model and closes the ~15-20 point expected score gap.


Add `dmm_build` alias that targets only the 4 prod binaries (server, indexer, forecaster, ncaa-feed) instead of building the entire workspace. Updated `dmm_backend`, `dmm_all`, `dmm_backfill`, and `dmm_listen` to use it.


### Enforce URL-safe slugs in mirror and group client libraries

- **Problem**: Mirror and group slugs accepted any string, including spaces and special characters (e.g. "Barracks Ballers"), making them unsuitable for URLs.
- **Fix**: Added client-side slug validation in both layers:
  - **Rust**: `validate_slug()` in mirror-importer rejects non-URL-safe slugs before writing platform.json. Also fixed default slug from `YAHOO-{id}` (uppercase) to `yahoo-{id}`.
  - **TypeScript**: `assertUrlSafeSlug()` in the client library validates slugs in `createMirror`, `addEntry`, `updateEntrySlug`, `createGroup`, and `createGroupWithPassword` before sending transactions.
- Slugs must be lowercase alphanumeric + hyphens (`[a-z0-9-]`), no leading/trailing hyphens.


Redesign live game display: replace hidden absolute-positioned indicator with prominent in-card banner showing clock, Watch link, and blinking dot. Flip score/probability order so score is rightmost.


Remove redundant checkmark/X icons from bracket game slots — green/red styling already conveys correct/wrong picks


Show champion icon, P(win), and E[score] columns on mobile leaderboard

- Champion column: show team logo icon only on mobile (no text), full name on desktop
- Forecast columns (ℙ / 𝔼): always visible with math-style headers on mobile, compact text
- No column header for champion on any screen size


Fix pre-lock forecaster to use full possession-level game simulation (NB/Poisson + Bayesian metric updates) instead of incorrect reach-probability shortcut that used marginal advance probabilities as conditional win probabilities.


Fix bracket overlay showing teams as "advancing" (green) in rounds they haven't won yet. A team that won R64 was incorrectly highlighted green in R32 before that game was played, because the check used `>= round` (has reached this round) instead of `> round` (has won through this round).


Change mirror entry slug format from bracket name to `{firstname}-{champion}`. Prompts user to resolve conflicts when multiple entries produce the same slug.


Add BracketUpdated event handling to listener and backfill, and add backfill-mirror subcommand to populate mirror entries from contract state.


Fix live game resolver to use current scores when clock/period data is missing (e.g. during halftime). Previously fell back to pre-game KenPom probability, ignoring the actual score entirely.


Add mirror leaderboard and bracket viewer pages at `/mirrors/id/:id` and `/mirrors/id/:id/bracket/:entrySlug`. Not linked from anywhere in the UI — must be navigated to directly. Also fix group leaderboard to show group-specific win probabilities instead of global pool forecasts.


Add clickable NCAA game links to live games in bracket view. The period/clock label (e.g. "2H 7:30") on live games now links to the NCAA March Madness Live page for that game. Bracket position IDs are stored in tournament.json via fetch-bracket.


Fix cursor showing as pointer on bracket games in view-only mode (post-lock home page and /bracket/:address viewer). Now correctly shows default cursor when brackets are disabled.


Fix leaderboard table column widths: constrain P(Win) and E[Score] to narrow fixed widths, give Champion column a stable fixed width with truncation so it doesn't shift between pages.


Suppress clippy::type_complexity warning in multi-pool simulation


Add client-side pagination and sortable columns (score, expected score, win probability) to the leaderboard page. Replaces card layout with a compact table.


Return rich BracketForecast objects from /forecasts endpoint. The forecaster now computes expected score (mean simulated final score) alongside win probability and writes full `{expectedScore, winProbability}` objects to Redis. Frontend consumes these directly — no more bps-to-object transform. Leaderboard labels simplified ("X% / Y pts" instead of "P(Win): X% / E[Score]: Y").


Fix leaderboard crash when forecast data loads. The `/forecasts` API returns `{address: basisPoints}` (plain integers) but the frontend expected `BracketForecast` objects. Now `useForecasts` transforms basis points into proper `BracketForecast` objects, and the E[Score] line is hidden when unavailable.


**ncaa-feed**: Fix bug where final games never get score corrections. Previously, once a game was marked Final, the feed skipped all further updates — including score corrections from the NCAA API. Now the feed keeps the Final status but still applies score updates when the API reports a different score.


Fix advancing (green) color bleeding into future rounds. Teams now only show as advancing in rounds they've actually reached based on win count, not in all rounds where the user picked them.


Clean up tournament status data flow. Remove `teamReachProbabilities` from `TournamentStatus` (now stored separately in `mm:probs`). Server deserializes `mm:games` into a typed `TournamentStatus` before serving. Frontend shows game clock (period + seconds remaining) for live games and derives win probabilities from per-team advance probs via `/team-probs` endpoint.


Add [ncaa] section to mappings.toml for NCAA API name aliases. The mapper now loads these aliases at startup, resolving mismatches like "South Fla." → "South Florida" and "Saint Mary's (CA)" → "Saint Mary's" that caused unresolved team name warnings.


Add Yahoo Fantasy bracket mirror importer pipeline: Rust binary fetches Yahoo API data and encodes brackets to bytes8 hex (`mirror-importer` crate), Bun script creates/updates BracketMirror entries on-chain, shell wrapper orchestrates both steps. Includes response caching, Yahoo-to-NCAA team name mappings, and idempotent on-chain mirroring.


Fix NCAA scoreboard API response parsing: the API changed `data.scoreboard` to `data.contests` and switched `score`, `seed`, and `startTimeEpoch` from strings to numbers. Also handle ordinal period strings ("1st", "2nd") in addition to numeric ("1", "2").


Add `--poll-interval` flag to ncaa-feed for fixed polling interval in seconds, overriding the adaptive active/pre-game intervals. Conflicts with `--requests-per-sec`.


Remove stale `--output-file` flag from ncaa-feed supervisor config. The binary now writes tournament status directly to Redis, not to a file.


Show "submit a bracket first" warning in Join Group, Create Group, and Public Groups join flows when the connected wallet has no on-chain bracket entry. Links to homepage for bracket submission. Reactive to wallet switches.


Remove SMS login method from Privy config (disabled on Privy side)


Multi-pool forecaster: Redis-only input, per-pool win probabilities

- Forecaster reads all inputs from Redis (entries, groups, mirrors, tournament status) instead of a JSON file
- Computes per-pool win probabilities: main contest, each group, each mirror
- Stores results as Redis HASH `mm:forecasts` with basis-point values (10000 = 100%)
- Writes per-team advance probabilities to Redis HASH `mm:probs`
- New server endpoints: `/forecasts/groups/s/:slug`, `/forecasts/groups/id/:id`, `/forecasts/mirrors/s/:slug`, `/forecasts/mirrors/id/:id`, `/team-probs`
- Uses rayon for parallel simulation across CPU cores
- Seed command now generates mirrors and team reach probabilities
- Breaking: `mm:forecasts` changed from STRING to HASH; forecast values changed from `BracketForecast` objects to basis-point u32 values

Audit fixes for the multi-pool forecaster follow-up PR

- add `--pre-lock` mode to the forecaster and remove the temporary `--status` override
- keep scoped forecast routes split across `/s/` and `/id/` so slug and ID paths never collide
- ensure the forecaster respects `--tournament-file` consistently and writes deterministic empty forecast/team-prob state


Bracket UI: fix Final Four overlap without horizontal scroll, add eliminated (red) and advancing (green) pick cascades, center bracket layout


Refactor deadline fetching to read submission deadline from chain instead of using a hardcoded constant. DeadlineCountdown now shows a loading state until the on-chain value is available.


Fix validation errors flashing after successful group create/join by resetting `attempted` state on success


Move "Join a Group" above "Create a Group" on the Groups page. Highlight the join form with an accent border when arriving via an invite link (?slug=...&password=...), and auto-switch to the Join tab on mobile. Clear highlight and URL params after successful join.


### Leaderboard table → card layout

- **web**: Replaced leaderboard `<table>` with full-width card list using `@fab-ui/card` (shadcn registry). Each entry is a horizontal card with rank, player, champion pick with ESPN team logo, forecast stats, and score.
- **web**: Added shadcn infrastructure (components.json, cn() utility, CSS variables mapped to brand palette).
- **web**: Leaderboard cards are 3/4 width centered on desktop, full-width on mobile. Top-3 entries get brighter gradient backgrounds.


Parallelize Monte Carlo simulation in `calculate_team_win_probabilities` using rayon parallel iterators. Each simulation now runs on its own thread with a thread-local RNG, and results are reduced via HashMap merge. This speeds up the Kalshi calibrator and all other callers (sim, forecaster) proportionally to available CPU cores.


Fix groups UI not updating after join/create/leave: wait for tx receipt, then hydrate group from on-chain data instead of relying on potentially stale API. Also remove inline member list from Your Groups (members are on the group leaderboard).


Add bracket count display to the desktop header, showing the total number of submissions to the left of the Faucet link.


Move group membership tracking from frontend localStorage to Redis. Add `mm:address_groups` reverse mapping (address → group IDs) maintained by the indexer on join/leave events. New server endpoint `GET /address/:address/groups`. Frontend now fetches membership from API; localStorage only stores passphrases (client-side secrets).


Fix public groups showing as free: add entry_fee to indexer → Redis → server API pipeline

The GroupCreated event doesn't include entryFee, so the indexer now reads it from the
contract via getGroup() after seeing the event. The field flows through GroupData (Redis),
GroupResponse (server API), and is consumed by the frontend's usePublicGroups hook.


### 2026-03-18 — Add indexer seed command + group leaderboard

- **indexer**: New `seed` subcommand writes fake entries, tournament status, and groups to Redis for local dev. Supports `--entries N` and `--clean` flags.
- **web**: Added `/groups/:slug/leaderboard` and links from joined group cards and public group cards.
- **web**: Group leaderboards now show submitted addresses and tags before reveal, leaving score, max, forecast, champion, and bracket view blank until a revealed bracket exists.
- **web**: Invalid group leaderboard slugs now show an error page instead of falling back to the global leaderboard.
- **web**: Leaderboard-related API polling now uses React Query hooks instead of manual `useEffect` fetch loops.


### 2026-03-18 — Switch to @changesets/cli workflow

- **Workflow**: PRs now add individual `.changeset/*.md` files instead of editing `docs/changeset.md` directly. On merge to main, the `merge-changesets` GitHub Action collects entries, prepends them to `docs/changeset.md`, and deletes the individual files. Eliminates changeset merge conflicts.
- **CI**: Changeset check now verifies a `.changeset/*.md` file was added AND that `docs/changeset.md` was not directly modified. Both `ci.yml` and `ci.sh` updated.
- **Deps**: Added `@changesets/cli` and `@changesets/changelog-github` as dev dependencies.

### 2026-03-18 — Use PUSH_TOKEN in merge-changesets workflow

- **CI**: merge-changesets workflow now checks out with `PUSH_TOKEN` (fine-grained PAT) instead of the default `GITHUB_TOKEN`, allowing it to push to `main` past branch protection rulesets.


### 2026-03-18 — Stabilize Privy embedded wallet session sync (#186)

- **Wallet UX**: Unified Privy wallet selection logic across the app so embedded-wallet sessions consistently prefer the Privy-managed wallet over linked external wallets during refresh, chain sync, and logout.
- **Wallet UX**: Removed wagmi render-time wallet activation and moved wallet syncing to an effect-driven flow, eliminating the React hydration warning and preventing Google/embedded logout from falling through to MetaMask.
- **Bracket UX**: Deferred bracket local-storage hydration until wallet session setup has settled, and kept the submit bar in `Loading wallet...` until the shielded wallet actually matches the Privy-selected wallet. This removes the startup flicker through transient `Connect`/submit states and prevents the first `Load my bracket` click from incorrectly jumping to MetaMask.

### 2026-03-18 — Add Connect Wallet button to home page (#178)

- **Frontend**: Added a prominent "Connect Wallet" button on the home/bracket page, visible when the user is not connected. Uses the same Privy `login()` as the nav bar. Especially helpful on mobile where the hamburger menu isn't discoverable.

### 2026-03-18 — Fix encoded bracket display position and styling (#179)

- **Frontend**: Moved encoded bracket hex display from between "Reset Picks" and submit buttons to right of "Submitted" status badge on desktop.
- **Frontend**: Removed border from the encoded bracket hex element for a cleaner look.

### 2026-03-18 — Change default Kalshi calibrator edge threshold to $1000

- **bracket-sim**: Changed `--edge-threshold` default from $1.00 to $1000.00 in the `calibrate` binary. The previous default caused premature convergence on noise.

### 2026-03-18 — Fix maxPossible elimination cascade in partial scoring (#116)

- **Scoring**: `scoreBracketPartial()` now tracks elimination cascades for `maxPossible`. When a bracket's pick is wrong, downstream games that depend on that eliminated team are zeroed out of `maxPossible`, giving an accurate ceiling instead of an overstated one.
- **Tests**: Added comprehensive cascade tests: single wrong pick cascade, sibling branch isolation, path-specific cascading, multi-region cascades, R32+ cascades, all-R64-wrong total elimination, and coincidental match handling.

### 2026-03-18 — Restructure firstFour schema in tournament.json

- **Schema change**: `firstFour` is now `{ teams: [{ name, abbrev? }, ...], winner? }` instead of `[string, string]`.
- **fetch-bracket**: Detects FF game winners from NCAA API `isWinner` flag. Applies abbreviations to individual FF teams and builds combo abbreviation for the slot name.
- **ncaa-api**: Added `is_winner` field to `BracketTeam`.
- **bracket-sim**: Updated `TournamentJsonTeam.first_four` to new struct. KenPom averaging and FF→slot mapping use `ff.teams[].name`.
- **ncaa-feed mapper**: Updated FF name extraction to new schema.
- **scrape_kenpom.py**: Updated to read `firstFour.teams[].name`.
- **Frontend**: Added `FirstFourEntry` and `FirstFourTeam` TypeScript interfaces.
- **data/mappings.toml**: Added abbreviations for `Prairie View A&M` and `Miami (Ohio)`.

### 2026-03-18 — Move team abbreviations to data/mappings.toml

- **data/mappings.toml**: Added `[abbreviations]` section with short display names for long team names.
- **fetch-bracket**: Loads abbreviations from `mappings.toml`, writes `abbrev` field to `tournament.json` for teams whose name exceeds 9 characters. First Four combo names are never abbreviated (future iteration).
- **Frontend**: Removed hardcoded `ESPN_ABBREVIATIONS` map and `getTeamAbbreviation()` — abbreviations now come entirely from `tournament.json`.
- **Workspace**: Added `rustls-tls` feature to workspace `reqwest` (fixes HTTPS on machines where TLS wasn't unified from other crates). Made `toml` a workspace dependency.

### 2026-03-18 — Move tournament status from file to Redis (#44)

- **Breaking**: `ncaa-feed` now writes tournament status to Redis (`mm:games` key) instead of `data/{year}/men/status.json`. Removed `--output-file` CLI arg; added `--redis-url` (env: `REDIS_URL`).
- **Server**: `/tournament-status` endpoint reads from Redis instead of file-based TTL cache. Removed `--tournament-status-file` CLI arg.
- **Forecaster**: Added `--live` flag to read tournament status from Redis. Without it, falls back to `--status <path>` (file-based, default `data/{year}/men/status.json`).
- **bracket-sim**: Added `--live` flag to read tournament status from Redis for conditioned simulation. `--status <path>` still works for file-based input.
- **Shared**: Added `KEY_GAMES` constant (`mm:games`) to `redis_keys.rs`.

### 2026-03-17 — Condition simulation on live game state (#43)

- **New**: `Game::simulate_remaining()` in `bracket-sim/src/game.rs` — simulates only the remaining possessions of a live game from the current score, based on time remaining and period. Handles regulation remainder and overtime.
- **New**: `Game::conditional_win_probability()` — Monte Carlo estimation of P(team1 wins | current score, time remaining) using the KenPom-based game model.
- **Forecaster**: When live games have score + `secondsRemaining` + `period` data, the forecaster now computes model-derived conditional probabilities instead of relying on externally-set `team1WinProbability`. Loads team metrics from embedded KenPom data and traces bracket feeders to resolve which teams are playing in later rounds.
- **Forecaster**: Added `--year` CLI flag (default 2026) for selecting embedded tournament data.
- **Dependency**: `march-madness-forecaster` now depends on `bracket-sim` for game simulation.
- **Docs**: Updated `docs/api.md` with `secondsRemaining` and `period` field documentation.
- **Tests**: Added tests for `remaining_seconds`, `simulate_remaining` with big leads, close games, and end-of-game scenarios.

### 2026-03-17 — Tighten encoded bracket copy icon spacing

- **UI**: Reduced the dead space between the encoded bracket hex field and the copy/edit icon fan-out on desktop by replacing the fixed-width icon rail with a collapsing `max-width` transition.

### 2026-03-17 — Mobile create-bracket layout overhaul (space + no-scroll lanes)

- **Mobile UI**: Reclaimed top-of-page vertical space by moving the lock countdown into a compact inline mode beside "Brackets submitted" and removing the separate countdown card row.
- **Mobile UI**: Repositioned controls so `Reset bracket` sits directly under `Submit Bracket`. Moved hex tools out of the main card into a subtle bottom `0x` debug toggle with copy + paste support.
- **Mobile UI**: Reworked the mobile bracket tabs into stacked round lanes that fit screen width (no horizontal scrolling), with clearer matchup separation via per-game borders and reversed two-column lane order for visual flow.
- **Mobile UI**: Applied the same lane treatment to the Final Four tab; current order is `Final Four` above `Championship`, with champion summary shown below.

### 2026-03-17 — Move Reset Picks & hex input into SubmitPanel, center deadline

- **UI**: Moved "Reset Picks" button, ConfirmDialog, and hex contract input from HomePage into SubmitPanel (desktop: new row below main bar; mobile: same placement). All hex state/handlers now live in SubmitPanel.
- **UI**: Hex input is a fixed-width (`w-[10.5rem]`) input-styled container with single-click to edit (removed double-click fan-out flow). Copy button visible next to field when bracket exists.
- **UI**: DeadlineCountdown is now centered (`flex justify-center`) above BracketView.
- **Cleanup**: Removed unused `useCallback`, `useEffect`, `useRef` imports and stale hex/expand state from HomePage.

### 2026-03-17 — Prompt external wallet users to add/switch to the Seismic network

- **Wallet UX**: When an external wallet connected through Privy is on the wrong chain, the submit panel now shows a `Switch to ...` action and explains that MetaMask may need to add the Seismic network first.
- **Wallet UX**: The app now attempts `wallet_addEthereumChain` when the required Seismic network is missing, then switches the wallet to that chain automatically.
- **Fix**: After a successful external-wallet chain change, the active Privy/wagmi wallet is re-synced so the Seismic shielded wallet client refreshes without requiring a manual hard refresh.

### 2026-03-17 — Fix submit bracket button UX

- **UI**: Added `cursor-pointer` to the submit/update bracket button on both desktop and mobile so it shows the hand icon when hoverable.
- **Fix**: Submit button now correctly reflects wallet readiness — disabled until the wallet client is fully initialized, not just when Privy auth is active. Prevents silent failures when `authenticated` is true but the embedded wallet hasn't loaded yet.
- **Fix**: `submitBracket`, `updateBracket`, and `setTag` now display a visible error message ("Wallet not connected") instead of silently failing when the wallet client isn't ready.

### 2026-03-17 — Bracket submission counter on home page

- **UI**: Added a bracket count indicator next to the deadline countdown on the home page, showing how many brackets have been submitted. Fetches from the `/stats` API endpoint, polls every 30s. Gracefully hidden when the API is unavailable.

### 2026-03-17 — Groups page polish

- **UI**: Simplified mobile tab labels: "Your Groups" → "Yours", "Public Groups" → "Public", "Join Group" → "Join", "Create Group" → "Create".
- **UI**: Moved "Browse Public Groups" button from right column (Your Groups) to left column (under Join Group section) on desktop.
- **UI**: Public group cards now show "Joined ✓" indicator instead of "Join" button for groups the user is already a member of.
- **UI**: Repositioned entry fee display in public group cards to be right-aligned between member count and Join button for consistent alignment.

### 2026-03-17 — Responsive Groups page with mobile tabs and desktop hub layout

- **UI**: Mobile (< 768px): tab-based layout with "Your Groups", "Public Groups", "Join Group", and "Create Group" tabs. Empty state in "Your Groups" links to the other tabs.
- **UI**: Desktop (>= 768px): two-column hub layout — left column has Create + Join forms stacked, right column has Your Groups list and a prominent "Browse Public Groups" link.
- **UI**: Added `/groups/public` route with a dedicated page for browsing public groups (linked from the desktop hub).

### 2026-03-17 — Restrict Privy login methods

- **UI**: Removed LinkedIn, Spotify, Instagram, TikTok, Apple, Farcaster, Telegram, and Passkey from the Privy login methods. Only Twitter, Discord, Google, GitHub, email, SMS, and wallet remain.

### 2026-03-17 — Show entry fee on public groups list

- **UI**: Public groups list now always displays the entry fee. Groups with no fee show "Free" instead of hiding the fee entirely.

### 2026-03-17 — Rename "Join Private Group" to "Join Group"

- **UI**: Renamed "Join Private Group" heading to "Join Group" and updated subtitle to be group-type-agnostic.
- **UI**: "Private group" checkbox now defaults to checked, so users joining via invite links get the passphrase field by default.

### 2026-03-17 — Fix public group joins failing when passphrase field is non-empty

- **UI**: Replaced the always-visible passphrase input with a "Private group" toggle checkbox. Passphrase field only appears when the toggle is on, and `handleJoin()` uses the toggle state (not input text or API-resolved group type) to choose between `joinGroup` and `joinGroupWithPassword`. This eliminates the bug where leftover passphrase text caused public group joins to revert with `GroupIsNotPasswordProtected`.
- **UI**: Removed `resolvedGroupNeedsPassword` state and associated pre-check logic. The user explicitly controls whether to use the password path.
- **UI**: Invite links with `?password=...` query params still work — the toggle defaults to ON when `initialPassphrase` is provided.

### 2026-03-17 — Reorganize Groups page with public groups browse and search

- **UI**: Reorganized Groups page into four clear sections: Public Groups, Your Groups, Join Private Group, Create Group.
- **UI**: Added Public Groups section that fetches all groups from the backend API and filters to public (no password) groups.
- **UI**: Added client-side search bar for filtering public groups by name or slug.
- **UI**: Public groups display as cards with inline "Join" button that expands to show a display name input and confirm button.
- **UI**: Public Groups section works without wallet connected (browse-only, join button disabled).
- **UI**: Extracted `PrivateJoinForm` component for joining private groups via slug + passphrase.
- **UI**: Refactored `GroupsSection` to only show "Your Groups" section (hidden when no groups joined).
- **Hook**: Added `usePublicGroups` hook that fetches from `VITE_API_BASE/groups` API endpoint.

### 2026-03-17 — Add NCAA team logos to bracket UI

- **Web**: Added ESPN CDN team logos next to team names in the bracket UI. Logos appear on the outer edge (left for East/South, right for West/Midwest).
- **Web**: New `espn-logos.ts` mapping file with ESPN team IDs for all 68 tournament teams. First Four combo names gracefully show no logo.
- **Web**: Exported `TeamLogo` component with `onError` fallback for broken images. Champion display in FinalFour also shows the logo.
- **Web**: Added `<link rel="preconnect">` for ESPN CDN to speed up logo loading.

### 2026-03-17 — Add `member_count` to groups + `check-redis` subcommand

- **Indexer**: `GroupData` now tracks `member_count` field, updated atomically with `members` vec on join/leave.
- **Indexer**: Backfill sanity check now verifies all group `member_count` values match `members.len()`.
- **Indexer**: New `check-redis` subcommand for Redis-internal consistency checks (no RPC needed): default checks all, `--group <slug>` for a specific group, `--all-groups`.
- No API changes — existing `GET /stats` (HLEN, O(1)) and `GET /groups` (member_count) continue to work.

### 2026-03-17 — Fix group/mirror event ordering in indexer

- **Indexer**: Group events (GroupCreated, MemberJoined, MemberLeft) and mirror events (MirrorCreated, EntryAdded, EntryRemoved) are now sorted by `(block_number, log_index)` before processing, instead of being grouped by event type. Fixes edge case where leave-then-rejoin within a single poll cycle or backfill batch could produce incorrect state.

### 2026-03-16 — Use on-chain submission deadline instead of hardcoded constant (#113)

- **Web**: Added `useSubmissionDeadline` hook that reads `submissionDeadline()` from the MarchMadness contract, falling back to the hardcoded constant if the contract read fails.
- **Web**: `useContract` hook now exposes `submissionDeadline` (number, seconds) and a reactive `isBeforeDeadline` that updates every second.
- **Web**: `DeadlineCountdown` accepts an optional `deadline` prop (defaults to hardcoded constant for backward compat).
- **Web**: `SubmitPanel` derives lock state from `contract.isBeforeDeadline` instead of re-reading the hardcoded constant.
- **Web**: Fixes local dev mismatch where the UI deadline could differ from the deployed contract's deadline.

### 2026-03-16 — Add 90-day results submission deadline

- **Contracts**: Added `RESULTS_DEADLINE = 90 days` constant — owner must post results within 90 days of the submission deadline or the window closes.
- **Contracts**: `submitResults()` now reverts with `ResultsSubmissionWindowClosed` if called after the window.
- **Contracts**: Added `collectEntryFee()` — entrants can reclaim their entry fee if the owner misses the results window (no-contest escape hatch).
- **Contracts**: Added `hasCollectedEntryFee` mapping and `ResultsWindowStillOpen` error.
- **Tests**: Added `ResultsDeadline.t.sol` with 7 tests covering the deadline guard and refund mechanism.

### 2026-03-16 — Fix scoreboard null data for future dates

- **ncaa-api**: Treat missing `data`/`scoreboard` in NCAA API response as empty list instead of error. The API returns null for dates without game data (e.g. future dates), which is not an error condition.

### 2026-03-16 — Apply Seismic brand colors

- **UI**: Replaced generic indigo/dark-blue theme with Seismic brand palette (mauve `#825A6D`, dark purple `#523542`, warm grays, muted gold `#A6924D`).
- **UI**: Updated all `@theme` CSS variables in `index.css` for backgrounds, text, borders, accent, warning, and gold.
- **UI**: Added `--color-dark-purple` theme variable for secondary accent.
- **UI**: Replaced hardcoded `indigo-*`, `amber-*`, `yellow-*` Tailwind classes with semantic theme variables in GroupsSection, MirrorsSection, and HomePage.
- **UI**: Updated Privy login modal accent color to mauve.
- **Layout**: Fixed Final Four vertical centering and bracket region column alignment.

### 2026-03-16 — Fix NCAA schedule API breaking change

- **ncaa-api**: Updated schedule response parsing for NCAA API format change: `data.schedule` → `data.schedules.games`, `numberOfGames` → `count`, date format `YYYY/MM/DD` → `MM/DD/YYYY`.
- **ncaa-api**: `ContestDate::parse` now accepts both `YYYY/MM/DD` and `MM/DD/YYYY` formats.

### 2026-03-16 — Remove POST /tournament-status endpoint

- **Server**: Removed `POST /tournament-status` endpoint, `--api-key` CLI flag, and `TOURNAMENT_API_KEY` env var. The `ncaa-feed` crate writes `status.json` directly; the server only needs to serve it via GET.
- **Docs**: Updated `docs/api.md` and `CLAUDE.md` to reflect removal.

### 2026-03-16 — Add deploy configuration

- **Deploy**: Added `deploy/` directory with nginx, supervisor, and Redis setup for production.
- **nginx.conf**: Static frontend serving + reverse proxy `/api/*` to Rust server on port 3000.
- **supervisor.conf**: Process management for `server`, `indexer`, and `ncaa-feed`.
- **deploy/README.md**: Full deployment guide covering nginx + certbot SSL, supervisor, Redis (systemd), env vars, and build steps.
- **dotenvy**: All Rust binaries (server, indexer, ncaa-feed) now load `.env` from the repo root at startup. Supervisor no longer needs `environment=` lines.
- **Redis keys**: All Redis keys now prefixed with `mm:` (e.g. `mm:entries`, `mm:groups`) to namespace our data. Requires `FLUSHDB` or re-backfill after deploy.

### 2026-03-16 — Fix bracket picker clearing out-of-order picks (#121)

- **UI fix**: `clearDownstream` in `useBracket` now only clears downstream picks that chose the team from the changed game's side of the bracket. Picks for the other feeder team are preserved, allowing users to fill in brackets out of order without losing later-round selections.

### 2026-03-16 — Redis integration for chain metadata

- **Infra**: Replace flat JSON file storage with Redis for indexer and server.
- **Indexer**: Writes all chain events (entries, tags, groups, mirrors) to Redis instead of `data/entries.json`.
- **Indexer**: Contract addresses default to `data/deployments.json` if not specified via CLI.
- **Indexer**: Added event ABIs for BracketGroups (`GroupCreated`, `MemberJoined`, `MemberLeft`) and BracketMirror (`MirrorCreated`, `EntryAdded`, `EntryRemoved`).
- **Server**: Reads entries, groups, and mirrors from Redis. Removed file locking / TTL cache for chain data.
- **Server**: New endpoints: `GET /groups/:slug`, `GET /groups/:slug/members`, `GET /mirrors`, `GET /mirrors/:slug`, `GET /mirrors/:slug/entries`.
- **CI**: Added Redis service to GitHub Actions workflow and local `ci.sh`.
- **Config**: Added `REDIS_URL` to `.env.example` (default: `redis://127.0.0.1:6379`).
- **Deps**: Added `redis` 0.27 with `tokio-comp` + `aio` features to workspace.

### 2026-03-16 — Gitignore broadcast directory

- **Repo hygiene**: Removed committed `contracts/broadcast/` files and gitignored the entire directory. Broadcast logs are deployment artifacts that shouldn't be tracked.

### 2026-03-16 — Fix Kalshi trade log table alignment

- **Bug fix**: `Side` column (`BUY`/`SELL`) wasn't respecting formatter width — `write!(f, "BUY")` bypasses padding; switched to `f.pad(s)`.
- **Bug fix**: `Qty` column width was hardcoded to 4 but quantities can be 6+ digits; now computed dynamically via `log10`.
- **Change**: Removed `¢` symbols from Price/Model/Edge columns and `$` from EV column — values are plain numbers, units are in the header.

### 2026-03-16 — Redeploy BracketGroups with auto-join

- **Deploy**: Redeployed BracketGroups to testnet (`0xaDddc1fB51b771276B77c059a053153B7255280B`) with auto-join-on-create feature. MarchMadness and BracketMirror unchanged.

### 2026-03-16 — Auto-join creator when creating a BracketGroups group

- **Contract**: `createGroup` and `createGroupWithPassword` now auto-join the creator as the first member with default name "CREATOR". Both functions are now `payable` — creator sends the group entry fee (if any) with the transaction. Creator can update their name via `editEntryName`.
- **Client**: `createGroup` / `createGroupWithPassword` in `BracketGroupsUserClient` now automatically send `value: entryFee`.
- **Frontend**: `useGroups` hook now tracks the newly created group in localStorage immediately after creation (looks up group ID by slug).
- **Tests**: Updated all BracketGroups tests for auto-join behavior. Added tests for creator auto-join, name editing, and creation validation (no bracket, wrong fee, after deadline).

### 2026-03-16 — Prevent post-window BracketGroups scoring and add groups-only redeploy script

- **Bug fix**: `BracketGroups.scoreEntry()` now reverts once the main scoring window has closed, even if the member was already scored on `MarchMadness`. This prevents group winner state from changing after claims are live.
- **Tests**: Updated `BracketGroups.t.sol` to expect the closed-window revert for post-window group scoring.
- **Deploy**: Added `DeployBracketGroups.s.sol` plus `scripts/redeploy-bracket-groups.sh` to deploy only a new `BracketGroups` contract against an existing `MarchMadness` address and update only the `bracketGroups` field in `data/deployments.json`.
- **Tooling**: Added `bun run gen:abis`, backed by `scripts/generate-abis.ts`, to regenerate the checked-in client ABI snapshots directly from `ssolc` for `MarchMadness`, `BracketGroups`, and `BracketMirror`.

### 2026-03-16 — Simplify BracketMirror events to use slug instead of index

- **Contract**: BracketMirror events (`EntryAdded`, `EntryRemoved`, `BracketUpdated`) now emit `slug` (string) instead of `entryIndex` (uint256). Slug is the stable identifier; array index is an implementation detail that changes on swap-and-pop.
- **Deploy**: Added `DeployMirror.s.sol` forge script and `scripts/redeploy-mirror.sh` for redeploying only BracketMirror without touching MarchMadness or BracketGroups.

### 2026-03-16 — Fix submission deadline, reduce entry fee, redeploy contracts

- **Bug fix**: Submission deadline was March 18 at Noon — corrected to **March 19 at 12:15 PM EST** (1773940500).
- **Change**: Entry fee reduced from 1 ETH to **0.1 ETH** (testnet).
- **Deploy**: Redeployed all contracts (MarchMadness, BracketGroups, BracketMirror) to testnet.
- **Cleanup**: Deleted old broadcast artifacts from repo.
- **UX**: Entry fee display now says "testnet ETH" instead of just "ETH" to avoid confusion.

### 2026-03-16 — Remove dead Ethereum March Madness link from README

- **Docs**: Remove dead link to `github.com/EthereumMarchMadness` from Credits section.

### 2026-03-16 — Shared SlugInput component, fix track-by-slug width

- **Refactor**: Extract shared `SlugInput` component used by both join form and track-by-slug inputs — consistent sizing (`max-w-md`, `text-sm`, `py-1.5`).
- **UX**: Track-by-slug input now matches join form width instead of stretching full-width.

### 2026-03-16 — Reset picks dialog hints about reloading on-chain bracket

- **UX**: When user has an on-chain submission, the reset picks confirmation tells them they can reload it via "Load bracket" instead of "can't be undone".

### 2026-03-16 — Move Groups before Leaderboard in nav, track by slug

- **UX**: Reordered nav bar so Groups appears before Leaderboard (both desktop and mobile).
- **UX**: Replaced "Track by group ID" with "Track by slug" — looks up groups on-chain by slug instead of numeric ID, with error feedback.

### 2026-03-16 — Fix group join flow: entry fees, slug-first lookup, layout

- **Bug fix**: Joining a group with a non-zero entry fee now correctly populates the transaction value. Previously always sent 0.
- **UX**: Join flow now fetches group metadata before submitting the tx — validates group exists, checks wallet balance vs entry fee, and surfaces clear error messages.
- **UX**: Slug is now the primary lookup method (unambiguous). Numeric input falls back to ID lookup, but slug is tried first — fixes the "42069" slug ambiguity.
- **UX**: Join form inputs are now stacked vertically at full width so placeholder text isn't truncated.
- **UX**: "Track by ID" is separated into its own small section below the join form.
- **Data**: Entry fee, slug, and display name are now stored in localStorage for joined groups.

### 2026-03-16 — Remove Groups and Mirrors sections from homepage

- **Cleanup**: Removed GroupsSection and MirrorsSection from the homepage. Groups already have a dedicated `/groups` page. Mirrors will get a dedicated `/mirrors` page later (see issue).
- **No functional change**: MirrorsSection component is kept in the codebase for future use.

### 2026-03-16 — Add passphrase field and invite links for private group joining

- **Fix**: Private groups were impossible to join from the UI — no passphrase input existed. Added a passphrase field to the "Join a Group" form.
- **UX**: When a slug resolves to a password-protected group, the passphrase field highlights and an error message prompts the user to enter the passphrase.
- **Feature**: Shareable invite links for private groups (e.g., `/groups?slug=seismic-team&password=Quake100`). URL query params auto-populate the slug and passphrase fields. Invite link shown with copy button in the joined groups list for private groups.

### 2026-03-16 — Auto-fix sentinel bit on pasted hex brackets

- **Behavior change**: When a user pastes a bracket hex with a missing sentinel bit (bit 63), the UI now automatically flips the bit to make it valid instead of loading the invalid hex as-is.
- **UX**: Warning message now shows both the original pasted hex and the corrected hex so the user knows exactly what changed.

### 2026-03-16 — Make MirrorsSection discoverable with track/untrack UI

- **Problem**: MirrorsSection was completely invisible — it only rendered when `mirrorIds.length > 0`, and the only way to add mirror IDs was programmatically via localStorage. No input, no form, no way for users to discover or use mirrors.
- **Fix**: Always show MirrorsSection when a mirror contract is deployed. Added a "Track Mirror" form (accepts mirror ID or slug, validated on-chain) that saves to localStorage. Added "Untrack" button on each tracked mirror. Made `mirrorIds` state reactive so tracking/untracking updates the UI immediately.

### 2026-03-16 — Improve tag input width and submit button prominence on desktop

- **UX**: Widened desktop tag input from `w-32` (8rem) to `w-52` (13rem) so longer display names like "DRAPPIS LOVELY CHALK" are fully visible.
- **UX**: Added a vertical divider between the tag section and the submit/update button to visually separate the two actions.
- **UX**: Made the submit/update button more prominent: larger padding (`px-6 py-2`), bigger text (`text-sm`), and a subtle accent ring (`ring-2 ring-accent/30`) so it's harder to miss.

### 2026-03-16 — Add Groups page, nav link, and create-group UI (Fixes #82)

- **New page**: `/groups` route with dedicated `GroupsPage` — create groups (public or private with passphrase), set entry fee, auto-generated slug from display name.
- **Navigation**: Added "Groups" link to both desktop nav bar and mobile hamburger menu in `Header.tsx`.
- **Layout fix**: Constrained the join-group form in `GroupsSection` to `max-w-lg` with compact inline inputs on desktop, fixing the too-wide layout from issue #82.
- **Discoverability**: Empty-state text now links to the Groups page so users know where to create groups.

### 2026-03-16 — Add confirmation dialog to Reset Picks button

- **Frontend**: Clicking "Reset Picks" now shows a confirmation dialog ("This will clear all 63 picks. This can't be undone.") before clearing the bracket.
- Added `@headlessui/react` for accessible, headless dialog/modal components styled with Tailwind.
- New reusable `ConfirmDialog` component supports title, description, danger styling, and backdrop dismiss.

### 2026-03-16 — Remove redundant entry count from Header

- **Cleanup**: Removed the "1 entry" / entry count badge from both desktop and mobile Header since each user only has one entry, making the display redundant.
- Removed `entryCount` prop from `Header` component and removed the `useContract` hook from `App.tsx`.

### 2026-03-16 — Add copy/edit fan-out icons on hex display double-click

- **Frontend**: Double-clicking the bracket hex value now fans out a copy icon and an edit (pencil) icon instead of immediately opening the hex input. Copy writes `bracket.encodedBracket` to clipboard with "Copied!" feedback; edit opens the existing hex input easter egg. Icons auto-collapse after 3 seconds or on click outside. Smooth `max-w` + opacity transition for the fan-out animation.

### 2026-03-16 — Fix bracket-sim ByteBracket encoding to match contract (MSB-first)

- **Bug**: `bracket-sim` encoded game outcomes LSB-first (game 0 → bit 0) while `ByteBracket.sol` and the TS client use MSB-first (game 0 → bit 62, sentinel at bit 63). Hex strings from the sim decoded as "mostly 16 seeds win" in the UI because all bit positions were reversed.
- **Root cause**: bracket-sim was self-consistent (LSB encoding + LSB scoring) so its internal roundtrip tests passed. The golden test vectors from issue #63 were never added to bracket-sim, so the cross-language mismatch went undetected.
- **Fix**: Rewrote `to_byte_bracket_bb`, `from_byte_bracket_bb`, and `simulate_tournament_bb` to use MSB-first bit ordering via new `game_bit(i) = 1 << (62-i)` helper. `score_base_bb` now delegates to `seismic_march_madness::scoring::score_bracket` (direct port of on-chain `getBracketScore`). Added sentinel bit helpers (`set_sentinel`, `strip_sentinel`, `assert_sentinel`, `format_bb`, `parse_bb`). Added golden vector tests for encoding, scoring, and self-score to bracket-sim.
- **Breaking**: All hex strings from bracket-sim are now `0x`-prefixed lowercase with sentinel bit set. Downstream consumers parsing bare uppercase hex must update.

### 2026-03-16 — Show encoded bracket hex when picks are complete

- **Frontend**: The faint `0x` easter egg placeholder now shows the full encoded bracket hex (e.g. `0xad551133fffdfdff`) once all 63 picks are made. Slightly more visible than the empty `0x` hint. Still double-clickable to open the hex input for loading a different bracket.

### 2026-03-16 — Fix hex input paste not working

- **Bug**: Pasting a bracket hex into the easter egg input did nothing — three root causes:
  1. Used `validateBracket()` which requires the sentinel bit (first nibble >= 8), but bracket hex from simulations/tools often omits it. The sentinel is only needed for on-chain submission, not for loading picks.
  2. `onBlur` handler captured stale `hexInput` state (always `""` from initial render), so any blur event immediately closed the input
  3. Only relied on `onChange` for paste detection, which can be unreliable
- **Fix**: Replaced `validateBracket` with a simple `0x` + 16 hex char regex (no sentinel requirement). Added dedicated `onPaste` handler that reads directly from `clipboardData`. Fixed `onBlur` to check `hexRef.current.value` (DOM truth) instead of stale React state. Strips non-hex characters from pasted text.

### 2026-03-16 — Fix blank team names in bracket UI

- **Bug**: Team names were blank because `BracketGame` rendered `team.abbrev` but `tournament.json` has no `abbrev` field — only `name`, `seed`, `region`. The value was `undefined`, rendering as empty text with no console error.
- **Fix**: Made `abbrev` optional in the `Team` interface and added `team.abbrev ?? team.name` fallback in `BracketGame` so team names always display.

### 2026-03-16 — Bracket hex input easter egg

- **Frontend**: Added a hidden hex input next to the Reset Picks button. A faint `0x` hint is visible but not editable — double-click it to unlock the input field. Type or paste a valid bytes8 bracket hex string to auto-fill all 63 picks instantly. Input closes on blur (if empty) or on successful load. Only visible before the deadline.

### 2026-03-16 — Skip First Four teams in Kalshi calibration

- **Calibrate binary**: First Four teams (e.g. Texas, NC State) are now excluded from Kalshi market-making calibration. Kalshi has separate individual markets for each FF team, not a joint market for the bracket slot. Including them produced nonsense combined-name URLs and incorrect calibration signals. FF teams conservatively keep goose=0.
- **Filtering**: FF teams are filtered out at the market-selection step (before orderbook fetching), avoiding wasted API calls. A safety guard in the orderbook-to-TeamOrderbook loop catches any that slip through.

### 2026-03-16 — Improve calibrator trade table alignment

- **Trade log table**: Moved "Team" to the first column and "Side" to the second column for better readability. Added extra spacing between all columns so the table is less cramped.
- **Rust fmt**: Fixed a pre-existing `rustfmt` issue in `calibrate.rs`.

### 2026-03-16 — Filter Kalshi calibration to tournament teams only

- **Calibrate binary**: Markets are now filtered to only tournament teams (68) before fetching orderbooks, instead of fetching all ~150 markets per round. Cached orderbooks are also filtered by ticker on load.
- **Mappings**: Added 6 missing Kalshi → NCAA name mappings to `data/mappings.toml` `[kalshi]` section: California Baptist, Hawai'i, LIU, Miami (OH), North Carolina St., Queens University.

### 2026-03-16 — Refresh ratings wrapper script

- **New script** `scripts/refresh-ratings.sh` — convenience wrapper that scrapes KenPom ratings then runs Kalshi calibration in sequence. Defaults to 2-hour Kalshi cache TTL. Flags: `--cache-ttl` (seconds), `--no-kalshi` (kenpom only), `--no-kenpom` (calibrate only). Everything after `--` passes through to the calibrate binary. Step indicators `[1/2]`/`[2/2]` show progress; kenpom failure aborts before calibration.

### 2026-03-16 — Unsquish First Four teams in KenPom CSV

- **Data**: `data/2026/men/kenpom.csv` now has one row per team (68 rows) instead of squishing First Four pairs into single rows with averaged metrics. Re-scraped from KenPom to get real individual ratings.
- **Calibration**: New `save_kenpom_csv_with_goose()` in `bracket-sim/src/team.rs` preserves individual team metrics when writing calibrated goose values. For First Four teams, the slot's calibrated goose is applied to both individual team rows.
- **Calibrate binary**: Updated to use `save_kenpom_csv_with_goose()` so calibration round-trips don't lose individual metrics.
- **No changes needed** to `load_teams_from_json` — it already looked up individual FF team names and averaged them for simulation.

### 2026-03-16 — Multi-contract support: Groups, Mirrors across client, UI, and server (closes #65)

- **Client library** (`packages/client`): Added ABIs (`abi-groups.ts`, `abi-mirror.ts`) and typed client wrappers for BracketGroups (`BracketGroupsPublicClient`, `BracketGroupsUserClient`) and BracketMirror (`BracketMirrorPublicClient`, `BracketMirrorAdminClient`). All group lifecycle methods exposed: createGroup, joinGroup, joinGroupWithPassword, leaveGroup, editEntryName, scoreEntry, collectWinnings, getGroupBySlug. Mirror methods: createMirror, addEntry, removeEntry, getEntryBySlug, etc. Updated barrel exports in `index.ts`.
- **Server** (`crates/server`): Added `GET /api/groups` stub endpoint returning an empty list (placeholder for future public group registry).
- **Web UI** (`packages/web`): Added `useGroups` hook with localStorage tracking of joined group IDs, group data refresh, and all group lifecycle methods. Added `GroupsSection` component displayed prominently on the home page for both pre- and post-lock states. Supports joining groups by ID or slug, leaving, editing display names, and tracking groups without on-chain join.

### 2026-03-16 — Embed tournament data in Rust lib via include_str! (closes #62)

- **New module** `crates/seismic-march-madness/src/data.rs` — embeds tournament.json and kenpom.csv for all available years (2025, 2026 men's) at compile time via `include_str!`. Year-parameterized API: `TournamentData::embedded(year)`, `KenpomRatings::embedded(year)`, `tournament_json(year)`, `kenpom_csv(year)`. No default year — callers must be explicit.
- **Updated `forecaster`** — `--tournament-file` is now optional; defaults to `TournamentData::embedded(2026)`.
- **Updated `ncaa-feed`** — `--tournament-file` is now optional; defaults to `GameMapper::load_embedded(2026)`. Mapper takes year parameter.
- **New dependency** `csv` on `seismic-march-madness` for KenPom CSV parsing.
- **Note**: `bracket-sim` is NOT updated — it continues reading from the filesystem. The embedded data is for external consumers who import `seismic-march-madness` without access to the repo's data files.

### 2026-03-16 — Cross-language golden test vectors for bracket encoding/scoring (closes #63)

- **New file** `data/test-vectors/bracket-vectors.json` — 8 golden bracket vectors (all-chalk, all-upsets, mostly-chalk, cinderella run, alternating, split regions, single-bit-flip, region boundary), 16 scoring tests against two result sets, and 6 validation tests. Shared source of truth for TypeScript, Rust, and Solidity.
- **Solidity tests** (`contracts/test/BracketVectors.t.sol`) — 30+ tests: self-score (192) for all 8 vectors, scoring against all-chalk and cinderella results (16 cross-checks), sentinel validation, e2e through MarchMadness contract (submit → results → score → payout), tied-winner pool splitting.
- **TypeScript tests** — extended `bracket.test.ts` with golden vector encoding, roundtrip, and validation tests. Extended `scoring.test.ts` with golden vector scoring and self-score tests.
- **Rust tests** — extended `crates/seismic-march-madness/src/scoring.rs` with golden vector encoding roundtrip, scoring, self-score, and validation tests.

### 2026-03-16 — Add @data/ TypeScript path alias for cleaner imports (closes #61)

- Added `@data/*` path alias in `packages/web/tsconfig.json` (paths) and `packages/web/vite.config.ts` (resolve.alias) pointing to the repo-root `data/` directory.
- Updated all `../../../../data/` relative imports in the web package to use `@data/` (constants.ts, tournament.ts).

### 2026-03-16 — Store bracket picks as hex in localStorage (closes #64)

- **Changed** `loadPicks` / `savePicks` in `packages/web/src/hooks/useBracket.ts` to use compact storage formats instead of JSON boolean arrays (~300+ chars).
- **Complete brackets** stored as canonical bytes8 hex string (18 chars, e.g. `0x8000000000000000`), using `encodeBracket` / `validateBracket` from the client library.
- **Partial brackets** stored as `"partial:"` + 63-char string of `1`/`0`/`-` (71 chars total), preserving in-progress picks across page refreshes.
- No migration needed — no real users yet; old JSON format is silently discarded on load.

### 2026-03-15 — Restructure data directory + centralized name mappings + First Four handling

- **Data directory restructure**: Moved from `data/{year}/` to `data/{year}/men/` and `data/{year}/women/`. All per-gender data (tournament.json, kenpom.csv, status.json, mappings/) now lives under a gender subdirectory. Renamed `tournament-status.json` → `status.json`. Updated all CLI defaults, path helpers, frontend imports, and test references.
- **New file** `data/mappings.toml` — centralized name mapping from sources (KenPom, Kalshi) to NCAA canonical names. Single source of truth for team name normalization.
- **Updated `scrape_kenpom.py`** — loads name mappings from `data/mappings.toml`, writes KenPom data with NCAA canonical names. Uses `tournament.json` (not bracket.csv) for `--bracket-only` filtering, expanding First Four entries to include both individual teams.
- **Updated `bracket-sim` team loading** — `load_teams_from_json()` now handles First Four entries by looking up both individual teams in KenPom and averaging their ratings (ortg, drtg, pace, goose).
- **Updated kalshi crate** — team name mapping now loads from centralized `data/mappings.toml` (removed `crates/kalshi/team_names.toml`).

### 2026-03-15 — Fix Kalshi orderbook parsing + calibration sensitivity

- **Fix** Kalshi API now returns `orderbook_fp` (string dollar format) instead of `orderbook` (integer cents). `OrderbookResponse` is now a `#[serde(untagged)]` enum supporting both legacy and FP formats. FP values converted to integer cents for downstream use.
- **Fix** Calibration `sensitivity` default changed from `2.0` to `0.001`. The old value was tuned for probability-based edges (small numbers), but real orderbook edges are in the thousands of dollars, causing goose values to slam to ±15 clamp on the first iteration.

### 2026-03-15 — Market-making calibrator (replaces CSV normalization pipeline)

- **New module** `crates/kalshi/src/orderbook.rs` — market-making edge computation against Kalshi orderbooks. Walks top N orderbook levels to compute buy/sell edge per team/round. Includes trade log printer with Kalshi URLs.
- **New types** in `crates/kalshi/src/types.rs` — `OrderbookLevel`, `Orderbook`, `TeamOrderbook`, `CachedOrderbooks`, `OrderbookResponse` for orderbook fetching and caching.
- **New REST methods** in `crates/kalshi/src/rest.rs` — `get_orderbook(ticker, depth)` fetches per-market orderbook (converts NO bids to YES asks), `get_round_orderbooks()` batch fetches, plus orderbook-specific cache (`load_orderbook_cache`/`save_orderbook_cache`).
- **New module** `crates/bracket-sim/src/calibration_mm.rs` — market-making calibration loop. Adjusts goose values to minimize trading edge against live orderbooks (signed edge as gradient). Converges when total edge < threshold.
- **Revamped** `calibrate` binary — fetches orderbooks in-process with cache, runs calibration loop, prints edge summary and top trades table. No more CSV normalization pipeline.
- **New dependency** `bracket-sim` → `kalshi` crate for in-process orderbook fetching (no intermediate CSV).
- **Deleted** legacy CSV calibration mode (`calibration.rs`), normalization pipeline (`fair_value.rs`, `kalshi` binary fetch/watch commands), and pipeline scripts (`refresh.sh`, `fit_kenpom_model.py`). The `MarketDef` type was trimmed to remove normalization-only fields (`expected_sum`, `floor_prob`).

### 2026-03-15 — Bracket fetcher: auto-populate tournament.json from NCAA API

- **New binary** `fetch-bracket` (in `ncaa-feed` crate) — queries the NCAA bracket API on Selection Sunday, extracts all 64 teams with seeds, regions, and Final Four pairings, then writes `data/{year}/tournament.json` and `data/{year}/mappings/ncaa-names.json`.
- **New module** `ncaa_api::bracket` — fetches tournament bracket from NCAA's `sdataprod.ncaa.com` GraphQL API (persisted query `get_championship_ncaa`). Returns typed `Championship` with games, regions, and teams.
- **Region ordering**: Automatically determines Final Four pairings by tracing `victorBracketPositionId` chains through regional finals → semifinals. Regions are ordered so indices 0,1 play each other and 2,3 play each other, matching the bracket encoding. For 2026: East-South, West-Midwest.
- **First Four handling**: Play-in games produce teams that feed into R64 slots. When an R64 game has only 1 team, the binary finds the First Four game that feeds into it and includes both competing teams. Output uses `firstFour: ["TeamA", "TeamB"]` field on affected slots. The `ncaa-names.json` maps both First Four team names to the same bracket position.
- **Real 2026 data**: Replaced fake tournament data with actual 2026 NCAA bracket (68 teams, 4 First Four games).
- Updated mapper tests to use real 2026 team names.

### 2026-03-15 — NCAA live score feed (closes #42, refs #43)

- **New crate** `crates/ncaa-api` — NCAA basketball API client. Rate-limited HTTP client for the NCAA GraphQL API with 429 exponential backoff. Fetches scoreboard (live/final/upcoming games) and schedule data. Basketball-only (MBB/WBB, Division 1). Strong types: `ContestState` enum (Pre, Live{period,clock}, Final(overtimes), Other(raw)), `Period` enum, `ContestDate`, parsed scores/seeds.
- **New crate** `crates/ncaa-feed` (`ncaa-feed` binary) — polls NCAA scoreboard, maps contests to bracket game indices (0-62), writes `data/2026/tournament-status.json`. Adaptive polling: pre-game (60s), active (configurable, default 1/s), auto-exit on tournament complete.
- **Game mapping**: Uses `data/2026/mappings/ncaa-names.json` (NCAA nameShort → bracket position). R64 fast path computes game index directly. Later rounds derive matchups from decided game winners.
- **Atomic writes**: tournament-status.json written via tmp+rename to prevent partial reads.
- **GameStatus fields**: Added `seconds_remaining: Option<i32>` and `period: Option<u8>` to `GameStatus` in `seismic-march-madness` types (per issue #43 spec for live game conditioning in simulations).
- **16 new tests**: 9 in ncaa-api (scoreboard parsing, clock/period/overtime parsing, team scores, contest date, sport codes), 7 in ncaa-feed (mapper positions, feeder games, name resolution, feed state, poll intervals, seeding from existing status).

### 2026-03-15 — Use custom errors instead of require strings in all contracts (closes #39)

- **MarchMadness.sol**: Replaced all ~15 `require(condition, "string")` statements with custom errors (`error ErrorName()` + `if (!condition) revert ErrorName()`). Errors with parameters: `IncorrectEntryFee(uint256 expected, uint256 actual)`.
- **BracketGroups.sol**: Replaced all ~20 `require` statements with custom errors. Errors with parameters: `IncorrectEntryFee(uint256 expected, uint256 actual)`.
- **BracketMirror.sol**: Replaced all ~10 `require` statements with custom errors.
- **All test files** updated to use `vm.expectRevert(ContractName.ErrorName.selector)` (or `abi.encodeWithSelector` for parameterized errors) instead of revert string matching.
- Errors defined per-contract (no shared error file) to keep things simple.

### 2026-03-15 — Improve desktop bracket vertical symmetry (closes #31)

- Replaced hardcoded pixel spacing (`getVerticalSpacing`) with flex-based layout using `justify-around` and `items-stretch`. Each round column now stretches to the same height as the R64 column, and games within each round automatically center between their two feeder games from the previous round.
- Top and bottom halves now use `items-stretch` for equal-height regions, producing a symmetric layout where the Final Four sits cleanly in the center.
- Added `gap-2` minimum spacing between games for visual breathing room.

### 2026-03-15 — Upgrade seismic foundry to nightly-94eb5fc (closes #15)

- Updated `sfoundry` pin in `mise.toml` from `nightly-08913bcc...` to `nightly-94eb5fc1...` (2026-03-14 release).

### 2026-03-15 — Make `score_base_bb` public in bracket-sim

- Removed `#[cfg(test)]` and `pub(crate)` gate from `scoring::score_base_bb` so downstream consumers (e.g. the brackets pool-strategy repo) can use it directly instead of duplicating the function.

### 2026-03-15 — Sim: configurable pace dispersion + score-dist calibration tool (closes #41)

- **Generalized pace distribution** in `crates/bracket-sim/src/game.rs` via `Game::sample_count(mean, d)` — a single dispersion ratio `d = variance/mean` controls the distribution family: d<1 uses binomial (underdispersed), d=1 uses Poisson, d>1 uses Gamma-Poisson/NB (overdispersed).
- **Unified regulation and OT paths** — overtime now uses the same pace distribution as regulation instead of the old fixed-pace workaround. The dispersion parameter naturally scales variance with the mean.
- **Calibrated default `DEFAULT_PACE_D = 0.3`** (underdispersed) via score-dist sweep against NCAA tournament empirical targets. At d=0.3 the simulated total-score stddev ≈ 20, closest to the empirical ~19.
- **Panic-free simulation** — all distribution constructors use `match` with deterministic fallbacks instead of `unwrap()`. No panics possible in `sample_count` or `simulate_with_pace`.
- **New CLI binary `score-dist`** — sweeps pace dispersion values and reports game-level statistics (avg total, margin spread, OT frequency, pace stddev) for calibration against empirical data.
- **New CLI flag `--pace-d`** on `sim` binary — overrides the default dispersion ratio.
- **Threaded `pace_d`** through `Tournament` (new field + `with_pace_d()` builder) → `Game::simulate()` → `Game::winner()` → `resolve_overtime()`.
- **New tests**: `sample_count_underdispersed`, `sample_count_overdispersed`, `sample_count_poisson_baseline`, `ot_has_pace_variance`.

### 2026-03-15 — Pipeline orchestration scripts

- **New script** `scripts/refresh.sh` — runs the full KenPom/Kalshi ingestion pipeline (scrape KenPom, fetch raw Kalshi futures, fit anchor model, normalize Kalshi futures, calibrate goose values). Supports `--hours N` flag to control cache TTL (default 6 hours).
- **CI: Python checks** — added `run_python` section to `scripts/ci.sh`: verifies `uv` deps install (`uv sync --frozen`) and runs `scrape_kenpom.py --help` as a smoke test. Wired into `all` and available as `./scripts/ci.sh python`.

### 2026-03-15 — Bracket simulation library and CLI

- **New crate** `crates/bracket-sim` — Poisson-based NCAA tournament simulation engine with Bayesian metric updates. Ported from private `brackets` repo with rand 0.8->0.9 migration for edition 2024 compatibility.
- **Library modules**: team loading/validation, game simulation (Poisson scoring + overtime), tournament orchestration, bracket encoding (ByteBracket u64 format), scoring systems, goose calibration against market odds.
- **CLI binary** `sim` — runs Monte Carlo tournament simulations and prints round-by-round advancement probabilities for all 64 teams.
- **CLI binary** `calibrate` — adjusts team "goose" ratings to match target probabilities (e.g. from Kalshi) using iterative Bayesian calibration with Beta posterior convergence checks.
- **Data files**: `data/2025/tournament.json`, `data/2026/tournament.json`, `data/{year}/kenpom.csv`.

### 2026-03-15 — BracketMirror + BracketGroups contracts

- **New contract** `BracketMirror.sol` — standalone admin-managed off-chain bracket pool mirror. No money, no scoring, no composition with MarchMadness. Entries have unique slugs within a mirror for URL-friendly lookup (`getEntryBySlug`). Swap-and-pop removal.
- **New contract** `BracketGroups.sol` — linked sub-groups composing with MarchMadness via `IMarchMadness` interface. Optional `sbytes12` password protection (shielded), optional entry fee with scoring + payout. Group IDs are `uint32`. Scoring delegates to `marchMadness.scoreBracket()` to avoid double work. Group struct uses `creator` (not `admin`). Join/leave gated by submission deadline.
- **New interface** `IMarchMadness.sol` — minimal 6-function interface (`hasEntry`, `submissionDeadline`, `resultsPostedAt`, `scoreBracket`, `scores`, `isScored`) so BracketGroups only needs the deployed address.
- **Deploy scripts**: `DeployAll.s.sol` (production) and `DeployAllLocal.s.sol` (local dev) deploy all 3 contracts. `deploy-testnet.sh` parses all 3 addresses and writes to `data/deployments.json`.
- **Frontend**: `constants.ts` exports `CONTRACT_ADDRESS`, `GROUPS_CONTRACT_ADDRESS`, `MIRROR_CONTRACT_ADDRESS` from `deployments.json` (handles both old string and new object formats).
- **Tests**: 35 BracketGroups tests (creation, join/leave, password, scoring delegation, payouts, deadline enforcement) + 24 BracketMirror tests (creation, entries, slug lookup, swap-and-pop, access control).
- **MarchMadness constructor**: Added `uint16 year` parameter — contracts are now self-describing for which tournament season they belong to. Deploy scripts pass year (production: `2026`, local: `YEAR` env var, default `2026`).

### 2026-03-15 — Kalshi odds ingestor crate

- **New crate** `crates/kalshi` — standalone Kalshi prediction market odds ingestor for March Madness futures. Fetches round-by-round win probabilities from Kalshi's REST API and WebSocket stream.
- **CLI binary** (`kalshi`) with two subcommands: `fetch` (one-shot REST fetch with file caching) and `watch` (live WebSocket NBBO streaming with periodic CSV writes).
- **Fair value computation**: microprice from order book pressure (bid/ask sizes), with fallback to midpoint. Normalizes probabilities per round, backfills missing teams, and enforces cross-round monotonicity.
- **Team name mapping**: `team_names.toml` maps Kalshi market names to canonical names.
- **Zero dependencies on other workspace crates** — fully standalone, can be used independently of the bracket simulation or forecaster.
- Ported from the `brackets` repo with edition 2024 compatibility fixes (removed explicit `ref` in implicitly-borrowing patterns, collapsed `if` blocks per clippy).

### 2026-03-15 — Python KenPom scripts + UV project

- **New**: `pyproject.toml` — UV Python project (`march-madness-scripts`) with dependencies for data pipeline scripts (cloudscraper, kenpompy, matplotlib, numpy, pandas, scikit-learn).
- **New**: `scripts/scrape_kenpom.py` — scrapes KenPom ratings via kenpompy + cloudscraper (Cloudflare bypass), outputs `data/{YEAR}/kenpom.csv` (team, ortg, drtg, pace). Supports `--bracket-only` filtering and `--seeds-from` bracket CSV.
- **New**: `scripts/fit_kenpom_model.py` — fits per-round logistic regression (degree-2 polynomial features, C=0.1 regularization) from KenPom stats to Kalshi market probabilities. Outputs `data/{YEAR}/kenpom_anchor_model.json` with model coefficients, scaler params, and anchor ranges. Generates fit quality plots to `data/{YEAR}/plots/`.
- **Updated**: `.gitignore` — added `.venv/`, `data/*/plots/`, `__pycache__/`.

### 2026-03-15 — Bracket forecaster: forward Monte Carlo win probabilities

- **New crate** `crates/forecaster` (`march-madness-forecaster`) — reads `data/entries.json` + `data/tournament-status.json` + `data/mens-2026.json`, runs forward Monte Carlo simulations (default 100k) to compute per-bracket win probabilities, writes `data/forecasts.json`.
- **Forward simulation**: resolves games round-by-round. Decided games use known winner, live games use in-game `team1WinProbability`, upcoming games derive P(A beats B) from `teamReachProbabilities` via Bradley-Terry: `P(A wins) = reach[A][r+1] / (reach[A][r+1] + reach[B][r+1])`. Later-round matchups depend on who actually advanced in each simulation — no independent coin-flip approximation.
- **Renamed** `crates/common` → `crates/seismic-march-madness` (`seismic-march-madness` crate). This is the shared library for types, scoring, simulation, and tournament helpers — importable by 3rd-party data providers.
- **Library consolidation**: Moved simulation engine (`simulate.rs`), tournament data loading (`tournament.rs`), and partial scoring helpers from the forecaster into the lib. Forecaster is now a thin CLI wrapper.
- **Library contents**: `scoring.rs` (ByteBracket scoring algorithm), `simulate.rs` (forward Monte Carlo), `tournament.rs` (bracket-order helpers, reach-prob builder, partial scoring), `types.rs` (all shared types).
- **Server**: Added `GET /api/forecasts` endpoint — serves `data/forecasts.json` with TTL cache (same pattern as entries/tournament-status).
- **Client types**: Added `BracketForecast` and `ForecastIndex` TypeScript types.
- **Leaderboard**: When forecasts are available, shows E[Score] and P(Win) columns. Win probability > 10% highlighted in green.
- **Frontend hook**: `useForecasts` — polls `/api/forecasts` every 30s.
- **API docs**: Added `docs/api.md` — full schema documentation with game index layout, all 64 team names, curl examples, Cargo.toml import snippet for the `seismic-march-madness` crate.
- **Server port**: Default port changed from 3001 → 3000 (matches nginx proxy config at `brackets.seismictest.net`).

### 2026-03-15 — Tournament Live UI: leaderboard, bracket viewer, scoring

- **Client library**: Ported ByteBracket scoring algorithm from Solidity to TypeScript BigInt (`scoring.ts`). Added `scoreBracket()` (full), `scoreBracketPartial()` (in-progress with max possible), `getScoringMask()`, `popcount()`, `pairwiseOr()`.
- **Types**: Added shared types in `packages/client/src/types.ts` — `TournamentStatus`, `GameStatus`, `EntryRecord`, `EntryIndex`, `PartialScore`.
- **Rust server**: Added `GET /api/tournament-status` (serves `data/tournament-status.json` with TTL cache) and `POST /api/tournament-status` (API key auth via `TOURNAMENT_API_KEY` env var or `--api-key` flag) for external data sources to push status updates.
- **React Router**: Added `react-router-dom` — routes: `/` (home/bracket picker), `/leaderboard` (scored entries), `/bracket/:address` (read-only viewer).
- **Leaderboard page**: Fetches entries + tournament status, scores each bracket with `scoreBracketPartial`, sorts by score. Shows rank, player tag/address, current/max score, champion pick, and link to bracket viewer.
- **Bracket viewer page**: Read-only bracket view at `/bracket/:address` with tournament status overlay. Breadcrumb nav back to leaderboard.
- **Tournament overlays on BracketGame**: Live games show pulsing green dot + basketball scores + win probability badge. Final correct picks show green checkmark. Final wrong picks show red X + strikethrough + muted opacity.
- **Header nav**: Added Bracket/Leaderboard navigation links (desktop inline, mobile in dropdown menu).
- **Seed data**: Created `data/tournament-status.json` with ~16 R64 finals, ~8 live games, rest upcoming, plus sample `teamReachProbabilities`.
- **Hooks**: `useTournamentStatus` (polls /api/tournament-status every 30s), `useEntries` (polls /api/entries every 30s), `useReadOnlyBracket` (compute GameSlot[] from hex string).

### 2026-03-15 — Desktop bracket layout redesign

- **Bracket convergence**: Fixed double-reversal bug in `BracketRegion` — right-side regions (West, Midwest) now correctly flow inward toward the center where the champion is crowned, matching standard bracket app convention.
- **Submit panel**: Redesigned as a compact horizontal bar on desktop (mobile layout unchanged). Progress, status, entry fee, tag input, and submit button all in one thin row.
- **Removed scoreboard footer**: Removed the placeholder scoreboard section — will revisit later.

### 2026-03-15 — Fix Buffer polyfill for Privy signing

- **Root cause**: Privy's embedded wallet signer calls `Buffer.from()` internally when signing EIP-712 typed data. `Buffer` is a Node.js global not available in browsers.
- Added `buffer` package as devDependency
- Added `Buffer` polyfill in `main.tsx` before any other imports
- Added `global: "globalThis"` to Vite config

### 2026-03-15 — Fix [object Object] in error display

- Error extraction now JSON.stringifies all non-string values so object details render as readable JSON instead of `[object Object]`
- Unrecognized error objects without standard fields are dumped in full

### 2026-03-15 — Mobile header dropdown + better error surfacing

- **Header**: On mobile, replaced inline buttons (Faucet, address, Connect/Disconnect) with a hamburger dropdown menu to prevent text overlap on small screens. Desktop layout unchanged.
- **Error handling**: Added `extractErrorMessage` helper that walks the error cause chain to surface the real error from Privy/viem instead of showing generic "An error has occurred" messages. Errors now show in a scrollable container on mobile.

### 2026-03-14 — Add `/checklist` skill

- Added `.claude/skills/checklist/SKILL.md` — user-invocable skill that mirrors the CLAUDE.md rules checklist, for quick verification before pushing or opening PRs
- Added sync note to CLAUDE.md: any changes to the rules checklist must also be reflected in the skill

### 2026-03-14 — Bracket spacing + sbytes8 fix (seismic-viem PR)

- **Bracket spacing**: Increased R64 vertical gap from 2px to 8px (desktop) / 6px (mobile), and bumped later rounds proportionally. Games no longer appear jammed together.
- **sbytes8 bug**: Root-caused to seismic-viem v1.1.1 missing `sbytes*` in `remapSeismicAbiInputs`. Fix submitted upstream: SeismicSystems/seismic#117 (v1.1.2). Will update dep when published.

### 2026-03-14 — Lazy signed read + hasEntry contract function

- **Contract**: Added `mapping(address => bool) public hasEntry` — set to `true` on `submitBracket()`. Allows anyone to check if an address has submitted without a signed read.
- **ABI + Client**: Added `getHasEntry(address)` to `MarchMadnessPublicClient`
- **Frontend**: On login, calls `hasEntry(address)` (public, no signing) to check submission status. The signed read (`getMyBracket`) is now only triggered when user clicks "Load my bracket" button.
- **SubmitPanel**: Shows "Load my bracket" button when `hasSubmitted` is true but bracket data hasn't been loaded yet
- **Integration test**: Added test for `hasEntry` (true for submitters, false for non-submitters)
- **Redeployed** contract to testnet: `0xD1cA8aDfdaE872D44Af5aACf8a9EfE7493c606cf`

### 2026-03-14 — UX improvements: faucet, localStorage picks, multi-round advancing

- **Faucet link**: added "Faucet" link in header (opens in new tab), plus a prominent `FaucetBanner` when connected with 0 ETH balance — shows address with copy button and "Get Testnet ETH" link
- **Copyable address**: connected wallet address is shown in header and clickable to copy on both mobile and desktop
- **Balance check**: `useContract` now fetches wallet ETH balance; App shows faucet banner when balance is 0
- **LocalStorage picks**: bracket picks persist in `localStorage` keyed by `mm-picks-{address}` (zero address for unauthenticated). On login, zero-address picks migrate to the real address if no existing picks. Picks survive page refresh.
- **Multi-round advancing**: users can now pick a team to advance multiple rounds without filling in their opponent's bracket path. E.g., click Duke → Duke → Duke all the way to the championship. Winner computation allows single-team picks; BracketGame enables clicking a team even when opponent is TBD.
- Added `FAUCET_URL` constant

### 2026-03-14 — Fix deadline timestamp + redeploy contract

- **Bug**: `SUBMISSION_DEADLINE` was `1742313600` (March 18, **2025**) instead of `1773853200` (March 18, **2026**). This caused the app to show "Brackets are locked" a year early.
- Fixed timestamp in `constants.ts` and `MarchMadness.s.sol`
- Redeployed contract to testnet: `0x9cf71ec28D89330fD537b9131752ADA8157622b5`
- Updated `CLAUDE.md` with correct timestamp and documented Seismic RPC millisecond-timestamp quirk
- **Privy connect**: configured Privy app with correct app ID, enabled all social login methods, and added `brackets.seismictest.net` to allowed domains in Privy dashboard

### 2026-03-14 — Mobile-friendly web app (closes #10)

- Added `useIsMobile` hook (viewport < 768px detection via matchMedia)
- BracketView: mobile renders tabbed region selector (East/West/South/Midwest/Final Four) instead of 1400px-wide horizontal layout
- BracketGame: added `mobile` prop — tighter padding (px-1.5, py-0.5), smaller text (11px), smaller min-widths (72-80px) so a full region fits on small screens
- BracketRegion: added `compact` prop — scaled-down vertical spacing between rounds for mobile
- Header: responsive text sizing, hidden entry count badge and address on small screens, shorter "Connect" label
- Scoreboard: responsive padding and font sizes
- App/SubmitPanel: tighter padding on mobile (px-2, p-4)
- Tested down to 320px width (iPhone SE) — each region tab fits without horizontal scroll

### 2026-03-14 — Deploy MarchMadness to Seismic testnet (gcp-2)

- Deployed MarchMadness contract to Seismic testnet (chain 5124): `0xEbc32b5436D7DaA0e5b79431074242a29890364b`
- Entry fee: 1 ETH, submission deadline: March 18, 2026 12:00 PM EST
- Updated `data/deployments.json` with testnet contract address
- Added `contracts/broadcast/` to `.gitignore` (sforge broadcast artifacts)

### 2026-03-14 — Single .env at repo root + testnet deploy script

- Consolidated all env vars into a single `.env` file at repo root (was also in `contracts/.env.example`)
- Added `.env` to root `.gitignore` — the file contains a real testnet deployer private key
- Created `.env.example` with documented placeholders for all env vars (deployment, frontend, local dev)
- Removed `contracts/.env.example` (no longer needed)
- Removed `.env` from `contracts/.gitignore` (root `.gitignore` handles it)
- Added `bun deploy:testnet` script — sources root `.env` for `DEPLOYER_PRIVATE_KEY` and `VITE_RPC_URL` (shared with frontend, no duplicate RPC var), runs sforge with the production deploy script
- Local populate script unchanged — still uses hardcoded anvil accounts, no `DEPLOYER_PRIVATE_KEY` needed
- Contract address resolution: `VITE_CONTRACT_ADDRESS` CLI override → `data/deployments.json` (checked-in, keyed by year + chain ID) → zero address fallback
- Added `data/deployments.json` — source of truth for deployed contract addresses, grouped by year (`{"2026": {"5124": "0x..."}}`). Written automatically by deploy script, easy to extend for 2027+.
- `bun deploy:testnet` runs `scripts/deploy-testnet.sh` — deploys via sforge, parses address, writes to `deployments.json`. Also supports `--contract-address 0x...` to skip deploy and just write the address.
- Populate script starts Vite dev server automatically after deploying, with `VITE_CONTRACT_ADDRESS` and `VITE_CHAIN_ID` injected. Use `--no-vite` to skip.
- Removed `VITE_PUBLIC_RPC_URL` — single `VITE_RPC_URL` used everywhere (wagmi transport + ShieldedWalletProvider public transport)
- Added `target/` to root `.gitignore`
- Updated CLAUDE.md, README.md, docs/technical.md with environment documentation

### 2026-03-14 — PR #8 Review: Restructure tests package to localdev (`packages/localdev`)

- Renamed `packages/tests` to `packages/localdev` (`@march-madness/localdev`) — this is primarily a local dev tool, not just tests
- Moved `integration.test.ts` from `src/` to `test/` directory (at same level as `src/`)
- Added shorthand bun scripts to root `package.json`: `bun p:pre`, `bun p:post`, `bun p:grading`
- Updated all references across CLAUDE.md, README.md, docs/technical.md, packages/mise.toml

### 2026-03-14 — PR #8 Review: Refactor tests to use client library (`packages/tests`)

- Refactored `populate.ts` and `integration.test.ts` to use `MarchMadnessPublicClient`, `MarchMadnessUserClient`, and `MarchMadnessOwnerClient` from `@march-madness/client` instead of raw `wallet.writeContract()` / `publicClient.readContract()` calls
- Added factory functions to `utils.ts`: `createMMPublicClient()`, `createMMUserClient()`, `createMMOwnerClient()`
- Removed local `ENTRY_FEE` constant from `utils.ts` — now re-exported from `@march-madness/client`
- Raw wallet calls kept only where client library cannot express the test (wrong entry fee, cross-user bracket read before deadline, non-owner submitResults)

### 2026-03-14 — Integration Tests & Local Dev Population (`packages/tests`)

- Added `src/utils.ts` — test utilities: random/chalky bracket generation, sforge deploy, sanvil process spawning, anvil account loader, seismic-viem client helpers, time manipulation (evm_increaseTime + evm_mine)
- Added `src/integration.test.ts` — full end-to-end test suite (expects sanvil already running): deploy via sforge, concurrent bracket submission, tags, updates, signed read (own bracket before deadline), fast-forward past deadline, transparent read, results posting, scoring, payout collection with balance verification
- Added `src/populate.ts` — local dev population script that spawns sanvil itself, deploys via sforge, and populates state:
  - `--phase pre-submission` (default): deploy with future deadline, no brackets (for testing submission UI)
  - `--phase post-submission`: deploy, submit all brackets concurrently, fast-forward, post results, score a few (for testing reveal/scoring UI)
  - `--phase post-grading`: everything above + score all + fast-forward past 7-day scoring window (for testing payout UI)
  - Sanvil is left running after the script completes so the frontend can use it
- Added `data/anvil-accounts.json` — all 10 standard anvil accounts with addresses, private keys, and labels
- Added `contracts/.env.example` — deployer key format for sforge script
- Added `tsconfig.json` to tests package, added typecheck/lint/build scripts to `package.json`
- Updated `packages/mise.toml` to include tests package in typecheck, lint, and build tasks

### 2026-03-14 — PR #5 Review Fixes

- provider.rs: Support both SeismicReth (prod) and SeismicFoundry (sanvil) via `IndexerProvider` enum and `--network` CLI flag
- ci.sh: Missing `Cargo.toml` or `cargo` now fails CI instead of silently skipping
- main.rs: Renamed `Check` enum variant to `SanityCheck` (CLI subcommand remains `check` via `#[command(name = "check")]`)

### 2026-03-14 — Rust Indexer Binary (`crates/indexer`)

- Built `march-madness-indexer` — event indexer for MarchMadness contract on Seismic
- Four subcommands via clap: `listen` (live polling), `backfill` (historical scan), `reveal` (post-deadline bracket reading), `check` (sanity check vs on-chain count)
- Uses seismic-alloy provider (`SeismicUnsignedProvider` via `SeismicProviderBuilder`) for all RPC calls
- `sol!` macro for type-safe ABI encoding/decoding of events (`BracketSubmitted`, `TagSet`) and contract calls (`getEntryCount`, `getBracket`)
- Replaced hand-rolled `rpc.rs` (raw reqwest JSON-RPC) with seismic-alloy provider in `provider.rs`
- File-based locking (fs2) for concurrent read/write safety with the server
- Index stored as BTreeMap keyed by lowercase hex address, written as pretty JSON to `data/entries.json`
- Graceful SIGINT shutdown for the listener
- Moved Cargo workspace from `crates/Cargo.toml` to repo root `Cargo.toml`
- Updated CI scripts and GitHub workflow to use root workspace

### 2026-03-14 — PR #6 Review Fixes (`packages/web`)

- Changed address truncation from first 8 + last 8 to first 4 + last 4 chars (e.g., `0x1234...abcd`)
- Replaced Inter font with Fira Mono as the global font (Google Fonts link + CSS body rule)

### 2026-03-14 — Max Privy Login Methods (`packages/web`)

- Expanded loginMethods from [twitter, discord] to all 15 Privy-supported methods: wallet, email, sms, google, twitter, discord, github, linkedin, spotify, instagram, tiktok, apple, farcaster, telegram, passkey
- Removed disableAllExternalWallets restriction to support MetaMask and other external wallets with wallet login method

### 2026-03-14 — Web Frontend Refactor: Use Client Library (`packages/web`)

- Deleted hand-written `src/lib/abi.ts` — ABI now comes from `@march-madness/client`
- Refactored `useContract` hook to use `MarchMadnessPublicClient` (transparent reads) and `MarchMadnessUserClient` (shielded writes, signed reads) from client library
- Replaced manual `walletClient.writeContract()` / `walletClient.readContract()` calls with client library methods (`mmUser.submitBracket()`, `mmUser.getMyBracket()`, etc.)
- Replaced duplicated `ENTRY_FEE` constant with import from `@march-madness/client`; `ENTRY_FEE_DISPLAY` now derived via `formatEther(ENTRY_FEE)`
- Bracket encoding already used `encodeBracket` from client library (no change needed)

### 2026-03-14 — Web Frontend (`packages/web`)

- Built React frontend with full 64-team bracket selection UI
- Added Privy authentication (Twitter, Discord, social logins) with embedded wallet via seismic-react ShieldedWalletProvider
- Bracket UI: 4 regions (East/West/South/Midwest) with visual progression through R64 → R32 → Sweet 16 → Elite 8 → Final Four → Championship
- Click-to-pick interface with automatic downstream clearing when changing picks
- Contract integration via `useContract` hook using `@march-madness/client` library (MarchMadnessPublicClient, MarchMadnessUserClient)
- Submission panel with progress bar (picks/63), entry fee display, tag/name input, encoded bracket preview
- Deadline countdown timer with lock detection (March 18, 2026 noon EST)
- Scoreboard placeholder for post-tournament scoring
- Dark theme with Tailwind CSS v4 (@tailwindcss/vite plugin)
- Env vars: VITE_PRIVY_APP_ID, VITE_CHAIN_ID, VITE_RPC_URL

### 2026-03-14 — Client Library Review Fixes (`packages/client`)

- Replaced hand-written ABI with exact sforge-generated ABI from `contracts/out/MarchMadness.sol/MarchMadness.json` (includes proper `sbytes8` types for shielded inputs)
- Refactored `MarchMadnessPublicClient` to use `getContract()` + `.read.functionName()` pattern (consistent with `UserClient`'s `getShieldedContract` pattern)
- Updated ABI test to verify `sbytes8` type on `submitBracket` and `updateBracket` inputs

### 2026-03-14 — Client Library (`packages/client`)

- Added `src/abi.ts` — MarchMadness contract ABI as const array (uses bytes8 for shielded types, seismic-viem handles shielding)
- Added `src/client.ts` — three-level client hierarchy:
  - `MarchMadnessPublicClient`: transparent reads (entry count, results, deadline, scores, tags)
  - `MarchMadnessUserClient`: shielded writes (submitBracket, updateBracket), signed reads (getMyBracket), transparent writes (setTag, scoreBracket, collectWinnings)
  - `MarchMadnessOwnerClient`: owner-only functions (submitResults)
- Added `src/format.ts` — human-readable bracket formatting (formatBracketLines, formatBracketJSON, getFinalFourSummary, getTeamAdvancements)
- Added `validateBracket(hex)` to `src/bracket.ts` — checks 0x prefix, hex length, and sentinel bit
- Fixed runner-up detection bug in `decodeBracket` — now correctly identifies the Final Four loser
- Updated `src/index.ts` barrel exports for all new modules
- Added tests: `abi.test.ts` (5 tests), `format.test.ts` (7 tests), expanded `bracket.test.ts` (8 new tests for validateBracket + runner-up)
- 25 total tests passing, typecheck clean

### 2026-03-14 — Rust HTTP Server (`crates/server`)

- Built `march-madness-server` HTTP server using axum + tokio
- Endpoints: `GET /api/entries` (full index), `GET /api/entries/:address` (single entry), `GET /api/stats` (total entries + scored count), `GET /health`
- TTL-cached reads of the indexer's JSON file (5s default) with fs2 shared/read file locks
- CORS enabled (Access-Control-Allow-Origin: \*) for frontend access
- CLI via clap: `--port` (default 3001) and `--index-file` (default `data/entries.json`)
- Graceful shutdown on SIGINT/SIGTERM
- Structured logging via tracing

### 2026-03-14 — CI Workflows + Local CI Script (mise-based)

- Added `mise.toml` (root) — pins sfoundry (nightly), ssolc (2ebb36d), bun (1.3.9) via mise, mirroring samlaf's setup in the seismic repo
- Added `contracts/mise.toml` — sforge tasks (build, test, fmt-check) with FOUNDRY_SOLC injection
- Added `packages/mise.toml` — bun tasks (typecheck, lint, build, test) for client and web
- Added `.github/workflows/ci.yml` — uses `jdx/mise-action@v2` for contracts and packages, cargo directly for crates, changeset enforcement on PRs
- Added `scripts/ci.sh` — local mirror of GitHub CI using mise, run before pushing
- Initialized crates workspace (common lib + indexer/server bins) and packages workspace (client + web + tests)
- Added CLAUDE.md rules #7 (every task ends with PR), #8 (ci.sh ↔ ci.yml sync), #9 (run CI locally before pushing)

### 2026-03-14 — Smart Contracts

- Added ByteBracket.sol library: ported jimpo's bit-manipulation scoring algorithm to Solidity 0.8 with bytes8 (unchecked blocks for bit ops)
- Added MarchMadness.sol main contract: shielded bracket storage (sbytes8), submit/update/score/payout lifecycle
- 57 tests pass with sforge

### 2026-03-14 — Initial Project Setup

- Created repo structure: contracts/, packages/, crates/, data/, docs/
- Added CLAUDE.md with project rules and architecture
- Added README.md with credits to jimpo and pursuingpareto (ByteBracket algorithm author)
- Tournament data in jimpo's format (name, teams, regions) — data/mens-2026.json
- Saved initial prompts to docs/prompts/
