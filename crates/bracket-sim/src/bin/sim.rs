use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::{DEFAULT_PACE_D, Game, Tournament, load_teams_for_year};
use clap::Parser;
use tracing::info;

use seismic_march_madness::{
    GameState, TournamentData, TournamentStatus, build_reach_probs, get_teams_in_bracket_order,
    run_team_advance_simulations,
};

const LIVE_GAME_SIMS: u32 = 10_000;

#[derive(Parser, Debug)]
#[command(name = "sim")]
#[command(version = "0.1.0")]
#[command(about = "Simulate tournament and print round-by-round advancement probabilities")]
struct SimArgs {
    /// Tournament year (determines bracket structure / Final Four pairings)
    #[arg(short = 'y', long, default_value_t = DEFAULT_YEAR)]
    year: u16,

    /// Path to combined teams CSV (overrides default JSON+KenPom loading)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Number of tournament simulations to run
    #[arg(short, long, default_value_t = 10000)]
    n_sims: usize,

    /// Pace dispersion ratio (variance / mean).
    /// <1 = underdispersed (binomial), 1 = Poisson, >1 = overdispersed (NB).
    #[arg(long, default_value_t = DEFAULT_PACE_D)]
    pace_d: f64,

    /// Path to tournament status JSON. When provided, conditions the simulation
    /// on decided and live game states instead of simulating from scratch.
    #[arg(long)]
    status_file: Option<PathBuf>,
}

/// Resolve which team index (0-63) won a decided game, tracing feeders back to R64.
fn resolve_winner_team_idx(g: usize, status: &TournamentStatus) -> Option<usize> {
    let game = &status.games[g];
    if game.status != GameState::Final {
        return None;
    }
    let winner_is_team1 = game.winner?;
    let round = game_round(g);
    if round == 0 {
        let (t1, t2) = (2 * g, 2 * g + 1);
        return Some(if winner_is_team1 { t1 } else { t2 });
    }
    let starts: [usize; 6] = [0, 32, 48, 56, 60, 62];
    let offset = g - starts[round];
    let prev = starts[round - 1];
    let (f1, f2) = (prev + 2 * offset, prev + 2 * offset + 1);
    let team1_idx = resolve_winner_team_idx(f1, status)?;
    let team2_idx = resolve_winner_team_idx(f2, status)?;
    Some(if winner_is_team1 {
        team1_idx
    } else {
        team2_idx
    })
}

fn resolve_game_teams(g: usize, status: &TournamentStatus) -> Option<(usize, usize)> {
    let round = game_round(g);
    if round == 0 {
        return Some((2 * g, 2 * g + 1));
    }
    let starts: [usize; 6] = [0, 32, 48, 56, 60, 62];
    let offset = g - starts[round];
    let prev = starts[round - 1];
    let (f1, f2) = (prev + 2 * offset, prev + 2 * offset + 1);
    let team1_idx = resolve_winner_team_idx(f1, status)?;
    let team2_idx = resolve_winner_team_idx(f2, status)?;
    Some((team1_idx, team2_idx))
}

fn game_round(g: usize) -> usize {
    let starts: [usize; 6] = [0, 32, 48, 56, 60, 62];
    for r in (0..6).rev() {
        if g >= starts[r] {
            return r;
        }
    }
    0
}

/// Compute model-derived win probabilities for live games and patch into status.
fn compute_live_game_probabilities(
    status: &mut TournamentStatus,
    team_names: &[String],
    team_map: &HashMap<String, bracket_sim::Team>,
    pace_d: f64,
) {
    for g in 0..63 {
        let game = &status.games[g];
        if game.status != GameState::Live {
            continue;
        }

        let (t1_idx, t2_idx) = match resolve_game_teams(g, status) {
            Some(pair) => pair,
            None => continue,
        };
        let t1_name = &team_names[t1_idx];
        let t2_name = &team_names[t2_idx];
        let (t1, t2) = match (team_map.get(t1_name), team_map.get(t2_name)) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };

        let game_model = Game::new(t1.clone(), t2.clone());

        let (prob, method) = if let (Some(score), Some(secs), Some(per)) =
            (&game.score, game.seconds_remaining, game.period)
        {
            let p = game_model.conditional_win_probability(
                (score.team1, score.team2),
                secs,
                per,
                pace_d,
                LIVE_GAME_SIMS,
            );
            (p, "conditional")
        } else {
            (game_model.team1_win_probability(), "pre-game")
        };

        info!(
            game_index = g,
            team1 = t1_name,
            team2 = t2_name,
            method,
            prob = format!("{:.3}", prob),
            "live game probability"
        );

        status.games[g].team1_win_probability = Some(prob);
    }
}

fn print_table(
    team_names: &[String],
    team_map: &HashMap<String, bracket_sim::Team>,
    probs: &[[f64; 6]],
) {
    println!(
        "\n{:<25} {:>4}  {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "Team", "Seed", "R64", "R32", "S16", "E8", "F4", "Champ"
    );
    println!("{}", "-".repeat(82));

    let mut indices: Vec<usize> = (0..64).collect();
    indices.sort_by(|&a, &b| {
        probs[b][5]
            .partial_cmp(&probs[a][5])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for &idx in &indices {
        let name = &team_names[idx];
        let seed = team_map.get(name).map(|t| t.seed).unwrap_or(0);
        println!(
            "{:<25} {:>4}  {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}%",
            name,
            seed,
            probs[idx][0] * 100.0,
            probs[idx][1] * 100.0,
            probs[idx][2] * 100.0,
            probs[idx][3] * 100.0,
            probs[idx][4] * 100.0,
            probs[idx][5] * 100.0,
        );
    }
}

fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .without_time()
        .init();

    let args = SimArgs::parse();
    let bracket_config = BracketConfig::for_year(args.year);

    let teams = load_teams_for_year(args.input.as_deref(), args.year)?;
    let team_map: HashMap<String, bracket_sim::Team> =
        teams.iter().map(|t| (t.team.clone(), t.clone())).collect();

    // Get team names in bracket order
    let tournament_data = TournamentData::embedded(args.year);
    let team_names = get_teams_in_bracket_order(&tournament_data);

    if let Some(status_path) = &args.status_file {
        // Conditioned mode: use forward sim with status file
        let status_str = std::fs::read_to_string(status_path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", status_path.display(), e)))?;
        let mut status: TournamentStatus = serde_json::from_str(&status_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: {}", status_path.display(), e),
            )
        })?;

        info!(
            year = args.year,
            n_sims = args.n_sims,
            pace_d = args.pace_d,
            "conditioned simulation (status file)"
        );

        // Compute conditional probabilities for live games
        compute_live_game_probabilities(&mut status, &team_names, &team_map, args.pace_d);

        // Build reach probs: use status file's if present, otherwise compute from Poisson sim
        let reach = if let Some(reach_map) = &status.team_reach_probabilities {
            if !reach_map.is_empty() {
                info!("using reach probs from status file");
                build_reach_probs(&team_names, reach_map)
            } else {
                compute_reach_probs(&teams, &bracket_config, &team_names, args.n_sims)
            }
        } else {
            compute_reach_probs(&teams, &bracket_config, &team_names, args.n_sims)
        };

        let results = run_team_advance_simulations(&status, &reach, args.n_sims as u32);
        let sims = results.num_sims as f64;

        let probs: Vec<[f64; 6]> = (0..64)
            .map(|idx| {
                let mut p = [0.0; 6];
                for (r, val) in p.iter_mut().enumerate() {
                    *val = results.advance[idx][r] as f64 / sims;
                }
                p
            })
            .collect();

        print_table(&team_names, &team_map, &probs);
    } else {
        // Unconditioned mode: full Poisson tournament sim (original behavior)
        info!(
            year = args.year,
            n_sims = args.n_sims,
            pace_d = args.pace_d,
            "unconditioned simulation"
        );

        let mut tournament = Tournament::new().with_pace_d(args.pace_d);
        tournament.setup_tournament(teams, &bracket_config);
        let win_probs = tournament.calculate_team_win_probabilities(args.n_sims);

        // Convert to bracket-order array
        let probs: Vec<[f64; 6]> = team_names
            .iter()
            .map(|name| {
                if let Some(raw) = win_probs.get(name) {
                    let mut cum = [0.0; 6];
                    for (r, val) in cum.iter_mut().enumerate() {
                        *val = raw[r..].iter().sum::<f64>();
                    }
                    cum
                } else {
                    [0.0; 6]
                }
            })
            .collect();

        print_table(&team_names, &team_map, &probs);
    }

    Ok(())
}

/// Compute reach probs from a full Poisson tournament sim.
fn compute_reach_probs(
    teams: &[bracket_sim::Team],
    bracket_config: &BracketConfig,
    team_names: &[String],
    n_sims: usize,
) -> seismic_march_madness::ReachProbs {
    info!(n_sims, "computing reach probs from Poisson sim");
    let mut tournament = Tournament::new();
    tournament.setup_tournament(teams.to_vec(), bracket_config);
    let cum_probs = tournament.cumulative_win_probabilities(n_sims);
    let reach_map: HashMap<String, Vec<f64>> = cum_probs.into_iter().collect();
    build_reach_probs(team_names, &reach_map)
}
