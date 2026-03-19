---
---

Fix forecaster expected scores by using full tournament simulation (simulate_tournament_bb with Bayesian metric updates) when all games are upcoming, instead of the forward sim which uses static metrics. This matches the oddsmaker's model and closes the ~15-20 point expected score gap.
