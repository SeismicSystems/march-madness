---
"bracket-sim": minor
---

Parallelize Monte Carlo simulation in `calculate_team_win_probabilities` using rayon parallel iterators. Each simulation now runs on its own thread with a thread-local RNG, and results are reduced via HashMap merge. This speeds up the Kalshi calibrator and all other callers (sim, forecaster) proportionally to available CPU cores.
