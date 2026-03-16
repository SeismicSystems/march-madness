# Tournament Status API

Base URL: `https://brackets.seismictest.net`

The server proxies `/api/*` to the Rust server on port 3000.

## Endpoints

### `GET /api/tournament-status`

Returns the current tournament status. No auth required.

### `POST /api/tournament-status`

Update the tournament status. **Requires API key.**

```
POST https://brackets.seismictest.net/api/tournament-status
Authorization: Bearer <TOURNAMENT_API_KEY>
Content-Type: application/json
```

#### curl example

```bash
curl -X POST https://brackets.seismictest.net/api/tournament-status \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d @tournament-status.json
```

### `GET /api/entries`

Returns all bracket entries (address → bracket hex + name). No auth required.

### `GET /api/forecasts`

Returns per-bracket win probabilities (written by the forecaster crate). No auth required.

---

## Tournament Status Schema

The POST body must be a JSON object with this shape:

```jsonc
{
  // REQUIRED: exactly 63 game statuses, indexed 0-62.
  "games": [
    // A finished game:
    {
      "gameIndex": 0,
      "status": "final",
      "score": { "team1": 82, "team2": 55 },
      "winner": true          // true = team1 (higher seed) won
    },
    // A live (in-progress) game:
    {
      "gameIndex": 12,
      "status": "live",
      "score": { "team1": 45, "team2": 38 },
      "team1WinProbability": 0.72   // conditional on current score, 0-1
    },
    // An upcoming game:
    {
      "gameIndex": 32,
      "status": "upcoming"
    }
    // ... 63 total
  ],

  // REQUIRED for forecasting: per-team probability of reaching each round.
  // Key = exact team name (must match data/mens-2026.json).
  // Value = array of 6 floats: [pR64, pR32, pS16, pE8, pF4, pChamp].
  //
  // pR64 is always 1.0 (every team starts in R64).
  // pChamp is the probability of winning the entire tournament.
  //
  // The forecaster derives pairwise win probabilities from these:
  //   P(A beats B in round r) = reach[A][r+1] / (reach[A][r+1] + reach[B][r+1])
  //
  // For eliminated teams (lost a "final" game), set remaining probs to 0.
  // You must include ALL 64 teams.
  "teamReachProbabilities": {
    "Duke":           [1.0, 1.0,  0.92, 0.78, 0.55, 0.31],
    "Washington":     [1.0, 0.0,  0.0,  0.0,  0.0,  0.0],
    "Michigan":       [1.0, 1.0,  0.90, 0.72, 0.48, 0.25],
    // ... all 64 teams
  },

  // Optional: ISO timestamp of when this data was generated.
  "updatedAt": "2026-03-20T18:30:00Z"
}
```

## Game Index Layout

The 63 games are indexed 0-62 in this order:

| Games    | Round         | Count | Matchups                        |
|----------|---------------|-------|---------------------------------|
| 0-31     | Round of 64   | 32    | Games 0-7: East, 8-15: West, 16-23: South, 24-31: Midwest |
| 32-47    | Round of 32   | 16    | Winner of games [2i, 2i+1]     |
| 48-55    | Sweet 16      | 8     | Winner of games [32+2i, 32+2i+1] |
| 56-59    | Elite 8       | 4     | Winner of games [48+2i, 48+2i+1] |
| 60-61    | Final Four    | 2     | Winner of games [56+2i, 56+2i+1] |
| 62       | Championship  | 1     | Winner of games [60, 61]        |

### R64 Matchups (games 0-31)

Each R64 game is between two teams in bracket order. Game `i` is between teams at positions `2*i` and `2*i+1` in bracket order.

**Team1** (higher seed) = even position, **Team2** (lower seed) = odd position.

Bracket order within each region is seed order: `[1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15]`.

So game 0 = 1-seed vs 16-seed (East), game 1 = 8-seed vs 9-seed (East), etc.

#### Full R64 game list

| Game | Region  | Team1 (higher seed)  | Team2 (lower seed)  |
|------|---------|----------------------|---------------------|
| 0    | East    | Duke (1)             | Washington (16)     |
| 1    | East    | Villanova (8)        | TCU (9)             |
| 2    | East    | Nebraska (5)         | Seton Hall (12)     |
| 3    | East    | Texas Tech (4)       | Stanford (13)       |
| 4    | East    | Saint Louis (6)      | Baylor (10)         |
| 5    | East    | Purdue (3)           | Tulsa (14)          |
| 6    | East    | Texas (7)            | Boise St. (11)      |
| 7    | East    | Michigan St. (2)     | UCF (15)            |
| 8    | West    | Michigan (1)         | Northwestern (16)   |
| 9    | West    | Iowa (8)             | Cincinnati (9)      |
| 10   | West    | Vanderbilt (5)       | Saint Mary's (12)   |
| 11   | West    | North Carolina (4)   | Virginia Tech (13)  |
| 12   | West    | BYU (6)              | Auburn (10)         |
| 13   | West    | Arkansas (3)         | New Mexico (14)     |
| 14   | West    | Santa Clara (7)      | Florida St. (11)    |
| 15   | West    | Iowa St. (2)         | Connecticut (15)    |
| 16   | South   | Arizona (1)          | N.C. State (16)     |
| 17   | South   | Clemson (8)          | Utah St. (9)        |
| 18   | South   | Wisconsin (5)        | SMU (12)            |
| 19   | South   | Alabama (4)          | West Virginia (13)  |
| 20   | South   | UCLA (6)             | VCU (10)            |
| 21   | South   | St. John's (3)       | Akron (14)          |
| 22   | South   | Louisville (7)       | Indiana (11)        |
| 23   | South   | Illinois (2)         | Georgia (15)        |
| 24   | Midwest | Florida (1)          | Miami FL (16)       |
| 25   | Midwest | Kentucky (8)         | South Florida (9)   |
| 26   | Midwest | Virginia (5)         | Oklahoma (12)       |
| 27   | Midwest | Kansas (4)           | San Diego St. (13)  |
| 28   | Midwest | Tennessee (6)        | Missouri (10)       |
| 29   | Midwest | Gonzaga (3)          | Grand Canyon (14)   |
| 30   | Midwest | Texas A&M (7)        | Ohio St. (11)       |
| 31   | Midwest | Houston (2)          | LSU (15)            |

## All 64 Team Names

These are the **exact** strings to use in `teamReachProbabilities`:

```
Duke, Washington, Villanova, TCU, Nebraska, Seton Hall, Texas Tech, Stanford,
Saint Louis, Baylor, Purdue, Tulsa, Texas, Boise St., Michigan St., UCF,
Michigan, Northwestern, Iowa, Cincinnati, Vanderbilt, Saint Mary's,
North Carolina, Virginia Tech, BYU, Auburn, Arkansas, New Mexico,
Santa Clara, Florida St., Iowa St., Connecticut, Arizona, N.C. State,
Clemson, Utah St., Wisconsin, SMU, Alabama, West Virginia, UCLA, VCU,
St. John's, Akron, Louisville, Indiana, Illinois, Georgia, Florida, Miami FL,
Kentucky, South Florida, Virginia, Oklahoma, Kansas, San Diego St.,
Tennessee, Missouri, Gonzaga, Grand Canyon, Texas A&M, Ohio St., Houston, LSU
```

## `winner` Field

- `true` = **team1** won (the higher-seeded / first-listed team in the matchup)
- `false` = **team2** won (the lower-seeded / second-listed team)
- Only set when `status` is `"final"`

## `team1WinProbability` Field

- Only relevant for `"live"` games
- Should be **conditional** on the current in-game score (not the pre-game probability)
- Range: 0.0 to 1.0

## Running the Server

```bash
# Start the server (default port 3000)
cargo run --bin march-madness-server

# Custom port
cargo run --bin march-madness-server -- --port 3000

# With API key for POST endpoint
TOURNAMENT_API_KEY=your-secret cargo run --bin march-madness-server
```

## Running the Forecaster

After tournament status is posted, run the forecaster to generate win probabilities:

```bash
cargo run --release --bin march-madness-forecaster

# Custom paths / simulation count
cargo run --release --bin march-madness-forecaster -- \
  --entries-file data/entries.json \
  --status-file data/2026/men/status.json \
  --tournament-file data/2026/men/tournament.json \
  --output-file data/forecasts.json \
  --simulations 100000
```

The forecaster reads `entries.json` + `data/2026/men/status.json` + `data/2026/men/tournament.json`, runs 100k Monte Carlo forward simulations, and writes `forecasts.json`. The server will pick up the new file within 5 seconds (TTL cache).

## Using the Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
seismic-march-madness = { git = "https://github.com/SeismicSystems/march-madness.git", path = "crates/seismic-march-madness" }
serde_json = "1"
```

Then construct and POST the `TournamentStatus`:

```rust
use seismic_march_madness::{TournamentStatus, GameStatus, GameState, GameScore};

let status = TournamentStatus {
    games: vec![
        GameStatus {
            game_index: 0,
            status: GameState::Final,
            score: Some(GameScore { team1: 82, team2: 55 }),
            winner: Some(true),
            team1_win_probability: None,
        },
        // ... 62 more
    ],
    team_reach_probabilities: Some(reach_map),
    updated_at: Some("2026-03-20T18:30:00Z".to_string()),
};

let json = serde_json::to_string_pretty(&status).unwrap();
// POST json to https://brackets.seismictest.net/api/tournament-status
```
