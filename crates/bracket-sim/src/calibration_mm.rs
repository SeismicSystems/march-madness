//! Market-making calibration: adjust goose values to minimize edge against Kalshi orderbooks.
//!
//! Instead of fitting to a normalized CSV of probabilities, this loop measures how much
//! money our model would make trading against the actual orderbook. An efficient model
//! (correct goose values) makes $0.

use std::collections::HashMap;
use tracing::{info, trace};

use kalshi::orderbook::{self, MarketEdge};
use kalshi::types::TeamOrderbook;

use crate::DEFAULT_KENPOM_UPDATE_FACTOR;
use crate::bracket_config::BracketConfig;
use crate::team::Team;
use crate::tournament::Tournament;

#[derive(Debug, Clone)]
pub struct MmCalibrationConfig {
    pub max_iterations: usize,
    pub sims_per_iteration: usize,
    /// Convergence threshold in dollars: stop when total edge < this.
    pub edge_threshold: f64,
    pub base_learning_rate: f64,
    pub decay_factor: f64,
    pub max_goose: f64,
    pub kenpom_update_factor: f64,
    /// Sensitivity: converts edge dollars to goose points.
    /// goose_delta = avg_edge_dollars * sensitivity * lr.
    /// With deep orderbooks (1000s of contracts), try ~0.001.
    pub sensitivity: f64,
}

impl Default for MmCalibrationConfig {
    fn default() -> Self {
        MmCalibrationConfig {
            max_iterations: 100,
            sims_per_iteration: 10_000,
            edge_threshold: 1000.0,
            base_learning_rate: 1.0,
            decay_factor: 0.3,
            max_goose: 15.0,
            kenpom_update_factor: DEFAULT_KENPOM_UPDATE_FACTOR,
            sensitivity: 0.001,
        }
    }
}

#[derive(Debug)]
pub struct MmCalibrationResult {
    pub converged: bool,
    pub iterations: usize,
    pub final_total_edge: f64,
    pub final_edges: Vec<MarketEdge>,
    pub goose_values: HashMap<String, f64>,
}

/// Run the market-making calibration loop.
///
/// Each iteration:
/// 1. Simulate tournament → cumulative advancement probabilities
/// 2. Build model_probs: HashMap<(team, round), f64>
/// 3. Compute total edge against orderbooks
/// 4. Check convergence (total_edge < threshold)
/// 5. Adjust goose per team using signed edge as gradient
/// 6. Zero-sum center + clamp
pub fn calibrate_mm(
    teams: &mut [Team],
    orderbooks: &[TeamOrderbook],
    config: &MmCalibrationConfig,
    bracket_config: &BracketConfig,
) -> MmCalibrationResult {
    let mut converged = false;
    let mut iteration = 0;
    let mut final_total_edge = 0.0;
    let mut final_edges = Vec::new();

    for iter in 0..config.max_iterations {
        iteration = iter + 1;
        let lr = config.base_learning_rate / (1.0 + iter as f64 * config.decay_factor);

        // 1. Simulate tournament
        let mut tournament =
            Tournament::new().with_kenpom_update_factor(config.kenpom_update_factor);
        tournament.setup_tournament(teams.to_owned(), bracket_config);
        let cum_probs = tournament.cumulative_win_probabilities(config.sims_per_iteration);

        // 2. Build model_probs from simulation results
        let mut model_probs: HashMap<(String, usize), f64> = HashMap::new();
        for (team_name, probs) in &cum_probs {
            for (round_idx, &prob) in probs.iter().enumerate() {
                let round = round_idx + 1; // 1-indexed
                model_probs.insert((team_name.clone(), round), prob);
            }
        }

        // 3. Compute total edge
        let (total_edge, edges) = orderbook::compute_total_loss(&model_probs, orderbooks);
        final_total_edge = total_edge;
        final_edges = edges;

        info!(
            iteration,
            total_edge = format_args!("${:.2}", total_edge),
            lr = format_args!("{:.4}", lr),
        );

        // 4. Check convergence
        if total_edge < config.edge_threshold {
            converged = true;
            break;
        }

        // 5. Compute per-team goose adjustment from edge
        // Positive edge → market values team higher → increase goose
        // Negative edge → market values team lower → decrease goose
        let mut goose_deltas: HashMap<String, (f64, f64)> = HashMap::new(); // (sum, count)

        for edge in &final_edges {
            if edge.edge.abs() < 0.001 {
                continue; // no meaningful edge, skip
            }
            let entry = goose_deltas.entry(edge.team.clone()).or_insert((0.0, 0.0));
            entry.0 += edge.edge;
            entry.1 += 1.0;
        }

        // 6. Apply goose adjustments
        for team in teams.iter_mut() {
            if let Some((signed_sum, count)) = goose_deltas.get(&team.team)
                && *count > 0.0
            {
                let avg_signed = signed_sum / count;
                let delta = avg_signed * config.sensitivity * lr;
                team.goose += delta;
                trace!(
                    team = %team.team,
                    signed_edge = format_args!("{:.3}", signed_sum),
                    delta = format_args!("{:+.4}", delta),
                    goose = format_args!("{:+.2}", team.goose),
                );
            }
        }

        // Zero-sum centering
        let mean_goose = teams.iter().map(|t| t.goose).sum::<f64>() / teams.len() as f64;
        for team in teams.iter_mut() {
            team.goose -= mean_goose;
            team.goose = team.goose.clamp(-config.max_goose, config.max_goose);
        }
    }

    let goose_values: HashMap<String, f64> = teams
        .iter()
        .filter(|t| t.goose.abs() > 1e-6)
        .map(|t| (t.team.clone(), t.goose))
        .collect();

    MmCalibrationResult {
        converged,
        iterations: iteration,
        final_total_edge,
        final_edges,
        goose_values,
    }
}
