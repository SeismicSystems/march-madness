use serde::Deserialize;
use statrs::distribution::{Beta, ContinuousCDF};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use tracing::{info, trace, warn};

use crate::bracket_config::{self, BracketConfig};
use crate::team::Team;
use crate::tournament::Tournament;

/// A target probability for a team at a specific round.
/// round: 1=Rd1, 2=Rd2, 3=Sweet16, 4=Elite8, 5=Final4, 6=Championship
#[derive(Debug, Clone, Deserialize)]
pub struct TargetOdds {
    pub team: String,
    pub round: usize,
    pub probability: f64,
}

#[derive(Debug, Clone)]
pub struct CalibrationConfig {
    pub max_iterations: usize,
    pub sims_per_iteration: usize,
    /// Beta posterior credible interval level for convergence. Converges when
    /// all targets fall within this CI. Higher = stricter (0.99 requires near-exact
    /// match given simulation noise, 0.90 converges faster but less precise).
    pub credible_level: f64,
    /// Initial goose adjustment magnitude per iteration. Higher = faster initial
    /// movement but more overshoot risk.
    pub base_learning_rate: f64,
    /// Controls how fast the learning rate decays: lr = base_lr / (1 + iter * decay).
    /// Higher = faster cooldown (settles sooner, may undershoot). Lower = stays
    /// aggressive longer (better for large corrections, may oscillate).
    pub decay_factor: f64,
    /// Clamp per-team goose to [-max_goose, max_goose] pts per 100 possessions.
    pub max_goose: f64,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        CalibrationConfig {
            max_iterations: 100,
            sims_per_iteration: 10_000,
            credible_level: 0.99,
            base_learning_rate: 1.0,
            decay_factor: 0.3,
            max_goose: 15.0,
        }
    }
}

#[derive(Debug)]
pub struct CalibrationResult {
    pub converged: bool,
    pub iterations: usize,
    pub final_errors: Vec<(String, usize, f64, f64)>, // (team, round, target, observed)
    pub goose_values: HashMap<String, f64>,
}

/// Check if target probability falls within the Beta posterior credible interval.
/// After observing k wins in n sims, posterior is Beta(k+1, n-k+1) with uniform prior.
fn target_within_credible_interval(
    observed_prob: f64,
    target_prob: f64,
    n_sims: usize,
    credible_level: f64,
) -> bool {
    let k = (observed_prob * n_sims as f64).round();
    let n = n_sims as f64;
    let alpha = k + 1.0;
    let beta_param = n - k + 1.0;
    let beta = Beta::new(alpha, beta_param).unwrap();
    let tail = (1.0 - credible_level) / 2.0;
    let lo = beta.inverse_cdf(tail);
    let hi = beta.inverse_cdf(1.0 - tail);
    target_prob >= lo && target_prob <= hi
}

pub fn load_targets_from_csv(path: &str) -> io::Result<Vec<TargetOdds>> {
    let file = File::open(path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut targets = Vec::new();
    for line in reader.deserialize() {
        let target: TargetOdds = line?;
        if !(0.0..=1.0).contains(&target.probability) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: round {} has probability {} (must be in [0.0, 1.0])",
                    target.team, target.round, target.probability,
                ),
            ));
        }
        if !(1..=6).contains(&target.round) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: round {} is out of range (must be in [1, 6])",
                    target.team, target.round,
                ),
            ));
        }
        targets.push(target);
    }
    Ok(targets)
}

/// Info about one bracket group's targets for a single round.
struct GroupTargets {
    round_idx: usize,
    /// Indices into the `targets` slice that belong to this group+round.
    indices: Vec<usize>,
    /// Sum of those probabilities.
    sum: f64,
    /// How many teams are in the bracket group (whether or not they have targets).
    group_size: usize,
}

const ROUND_NAMES: [&str; 6] = ["R1", "R2", "Sweet16", "Elite8", "Final4", "Championship"];

/// Iterate bracket groups and collect matching target indices + probability sums.
fn collect_group_targets(
    targets: &[TargetOdds],
    teams: &[Team],
    bracket_config: &BracketConfig,
) -> Vec<GroupTargets> {
    let groups = bracket_config::bracket_groups(teams, bracket_config);
    let mut result = Vec::new();

    for (round_idx, round_groups) in groups.iter().enumerate() {
        let round_num = round_idx + 1;
        for group in round_groups {
            let mut indices = Vec::new();
            let mut sum = 0.0;
            for (i, t) in targets.iter().enumerate() {
                if t.round == round_num && group.contains(&t.team) {
                    indices.push(i);
                    sum += t.probability;
                }
            }
            if !indices.is_empty() {
                result.push(GroupTargets {
                    round_idx,
                    indices,
                    sum,
                    group_size: group.len(),
                });
            }
        }
    }
    result
}

/// Renormalize target probabilities so that each bracket group sums to 1.0 per round.
/// `tolerance` is the max allowed deviation from 1.0 (e.g. 0.05 = +/-5%). Groups within
/// tolerance are rescaled; groups outside tolerance cause a panic.
/// Use tolerance >= 1.0 to renormalize unconditionally.
pub fn renormalize_targets(
    targets: &mut [TargetOdds],
    teams: &[Team],
    bracket_config: &BracketConfig,
    tolerance: f64,
) {
    let group_targets = collect_group_targets(targets, teams, bracket_config);

    // Check for groups outside tolerance before modifying anything
    let mut errors = Vec::new();
    for gt in &group_targets {
        if gt.sum <= 0.0 || gt.indices.len() < gt.group_size {
            continue;
        }
        let deviation = (gt.sum - 1.0).abs();
        if deviation > tolerance {
            let names: Vec<&str> = gt
                .indices
                .iter()
                .map(|&i| targets[i].team.as_str())
                .collect();
            errors.push(format!(
                "{}: [{}] sum {:.1}% deviates {:.1}pp from 100% (tolerance: +/-{:.0}%)",
                ROUND_NAMES[gt.round_idx],
                names.join(", "),
                gt.sum * 100.0,
                deviation * 100.0,
                tolerance * 100.0,
            ));
        }
    }
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("Error: {}", e);
        }
        panic!(
            "Renormalization failed: {} group(s) outside tolerance",
            errors.len()
        );
    }

    // Scale each group to sum to 1.0
    for gt in &group_targets {
        if gt.sum <= 0.0 {
            continue;
        }
        let scale = 1.0 / gt.sum;
        for &i in &gt.indices {
            targets[i].probability *= scale;
        }
    }
}

/// Validate target probabilities against bracket structure.
/// Returns (errors, warnings). Errors are fatal inconsistencies.
pub fn validate_targets(
    targets: &[TargetOdds],
    teams: &[Team],
    bracket_config: &BracketConfig,
) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    // 1. Monotonicity: P(advancing to later round) <= P(advancing to earlier round)
    let mut by_team: HashMap<&str, Vec<&TargetOdds>> = HashMap::new();
    for t in targets {
        by_team.entry(&t.team).or_default().push(t);
    }
    for (team, team_targets) in &by_team {
        let mut sorted = team_targets.clone();
        sorted.sort_by_key(|t| t.round);
        for window in sorted.windows(2) {
            if window[1].probability > window[0].probability + 1e-6 {
                errors.push(format!(
                    "{}: R{} prob ({:.1}%) > R{} prob ({:.1}%) -- can't be more likely to advance further",
                    team,
                    window[1].round,
                    window[1].probability * 100.0,
                    window[0].round,
                    window[0].probability * 100.0,
                ));
            }
        }
        if !teams.iter().any(|t| t.team == **team) {
            errors.push(format!("{}: not found in teams data", team));
        }
    }

    // 2. Bracket group sum constraints
    for gt in collect_group_targets(targets, teams, bracket_config) {
        if gt.indices.len() == gt.group_size && (gt.sum - 1.0).abs() > 0.05 {
            let names: Vec<&str> = gt
                .indices
                .iter()
                .map(|&i| targets[i].team.as_str())
                .collect();
            errors.push(format!(
                "{}: [{}] probabilities sum to {:.1}% (expected ~100%)",
                ROUND_NAMES[gt.round_idx],
                names.join(", "),
                gt.sum * 100.0,
            ));
        } else if gt.sum > 1.0 + 1e-6 {
            let names: Vec<&str> = gt
                .indices
                .iter()
                .map(|&i| targets[i].team.as_str())
                .collect();
            errors.push(format!(
                "{}: [{}] probabilities sum to {:.1}% (must be <= 100%)",
                ROUND_NAMES[gt.round_idx],
                names.join(", "),
                gt.sum * 100.0,
            ));
        }
    }

    (errors, warnings)
}

pub fn calibrate(
    teams: &mut [Team],
    targets: &[TargetOdds],
    config: &CalibrationConfig,
    bracket_config: &BracketConfig,
) -> CalibrationResult {
    // Group targets by team for efficient lookup
    let mut targets_by_team: HashMap<String, Vec<&TargetOdds>> = HashMap::new();
    for t in targets {
        targets_by_team.entry(t.team.clone()).or_default().push(t);
    }

    let mut converged = false;
    let mut iteration = 0;
    let mut final_errors = Vec::new();

    for iter in 0..config.max_iterations {
        iteration = iter + 1;
        let lr = config.base_learning_rate / (1.0 + iter as f64 * config.decay_factor);

        // Build tournament and run simulations
        let mut tournament = Tournament::new();
        tournament.setup_tournament(teams.to_owned(), bracket_config);
        let cum_probs = tournament.cumulative_win_probabilities(config.sims_per_iteration);

        // Compute errors and adjustments
        final_errors.clear();
        let mut all_within_ci = true;

        // Collect goose deltas per team
        let mut goose_deltas: HashMap<String, (f64, f64)> = HashMap::new(); // (weighted_sum, weight_sum)

        for (team_name, team_targets) in &targets_by_team {
            let observed_probs = match cum_probs.get(team_name) {
                Some(p) => p,
                None => {
                    warn!(team = %team_name, "target team not found in tournament");
                    continue;
                }
            };

            for target in team_targets {
                let round_idx = target.round - 1; // convert 1-indexed to 0-indexed
                if round_idx >= 6 {
                    warn!(team = %team_name, round = target.round, "invalid round");
                    continue;
                }

                let observed = observed_probs[round_idx];
                let error = target.probability - observed;

                let within_ci = target_within_credible_interval(
                    observed,
                    target.probability,
                    config.sims_per_iteration,
                    config.credible_level,
                );
                if !within_ci {
                    all_within_ci = false;
                }

                final_errors.push((
                    team_name.clone(),
                    target.round,
                    target.probability,
                    observed,
                ));

                // Sensitivity scales the raw error into a goose-point delta.
                // The 40.0 constant controls overall step magnitude (points per
                // 100 possessions per unit error). Dividing by round was intended
                // to dampen later rounds where goose compounds, but see note below.
                let sensitivity = 40.0 / target.round as f64;
                let delta = error * sensitivity * lr;

                // Weight by round -- intended to emphasize later rounds in the
                // weighted average. However, the round factor cancels out:
                //
                //   weighted_sum += (error * 40/round * lr) * round
                //                 = error * 40 * lr          (round cancels)
                //
                // So every round contributes `error * 40 * lr` to the numerator
                // regardless of round number. The denominator (weight_sum) still
                // depends on which rounds have targets, but within a single team's
                // targets, each round has equal influence on the final avg_delta.
                //
                // This is intentional: goose is a single team-level rating offset,
                // so there is no per-round knob to turn. Equal weighting avoids
                // over-fitting to late rounds that have noisier sim estimates.
                let weight = target.round as f64;
                let entry = goose_deltas.entry(team_name.clone()).or_insert((0.0, 0.0));
                entry.0 += delta * weight;
                entry.1 += weight;
            }
        }

        // Apply goose adjustments
        for team in teams.iter_mut() {
            if let Some((weighted_sum, weight_sum)) = goose_deltas.get(&team.team)
                && *weight_sum > 0.0
            {
                let avg_delta = weighted_sum / weight_sum;
                team.goose += avg_delta;
            }
        }

        // Zero-sum centering: subtract mean goose so total doesn't drift
        let mean_goose = teams.iter().map(|t| t.goose).sum::<f64>() / teams.len() as f64;
        for team in teams.iter_mut() {
            team.goose -= mean_goose;
            team.goose = team.goose.clamp(-config.max_goose, config.max_goose);
        }

        info!(
            iteration,
            all_within_ci,
            lr = format_args!("{:.4}", lr),
            mean_goose_correction = format_args!("{:.4}", mean_goose),
        );
        for (team_name, round, target, observed) in &final_errors {
            trace!(
                team = %team_name,
                round,
                target = format_args!("{:.3}", target),
                observed = format_args!("{:.3}", observed),
                error = format_args!("{:+.3}", target - observed),
            );
        }

        if all_within_ci {
            converged = true;
            break;
        }
    }

    let goose_values: HashMap<String, f64> = teams
        .iter()
        .filter(|t| t.goose.abs() > 1e-6)
        .map(|t| (t.team.clone(), t.goose))
        .collect();

    CalibrationResult {
        converged,
        iterations: iteration,
        final_errors,
        goose_values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_csv(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn load_targets_valid() {
        let f = write_csv("team,round,probability\nDuke,1,0.95\nDuke,2,0.80\n");
        let targets = load_targets_from_csv(f.path().to_str().unwrap()).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].team, "Duke");
    }

    #[test]
    fn load_targets_rejects_probability_above_one() {
        let f = write_csv("team,round,probability\nDuke,1,1.5\n");
        let err = load_targets_from_csv(f.path().to_str().unwrap()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let msg = err.to_string();
        assert!(msg.contains("Duke"), "error should name the team: {msg}");
        assert!(msg.contains("1.5"), "error should include the value: {msg}");
    }

    #[test]
    fn load_targets_rejects_negative_probability() {
        let f = write_csv("team,round,probability\nUNC,2,-0.1\n");
        let err = load_targets_from_csv(f.path().to_str().unwrap()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("UNC"));
    }

    #[test]
    fn load_targets_rejects_round_zero() {
        let f = write_csv("team,round,probability\nUNC,0,0.5\n");
        let err = load_targets_from_csv(f.path().to_str().unwrap()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("round 0"));
    }

    #[test]
    fn load_targets_rejects_round_seven() {
        let f = write_csv("team,round,probability\nUNC,7,0.5\n");
        let err = load_targets_from_csv(f.path().to_str().unwrap()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("round 7"));
    }
}

/// Pretty-print a table comparing target vs calibrated probabilities.
/// Sorted by target expected points descending.
pub fn print_calibration_table(final_errors: &[(String, usize, f64, f64)]) {
    const COLS: [&str; 6] = ["R32", "S16", "E8", "F4", "CG", "CW"];
    const WEIGHTS: [f64; 6] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
    const GW: usize = 13;

    // Group by team: (target, observed) per round
    let mut by_team: HashMap<String, [(f64, f64); 6]> = HashMap::new();
    for (team, round, target, observed) in final_errors {
        let entry = by_team.entry(team.clone()).or_insert([(0.0, 0.0); 6]);
        entry[round - 1] = (*target, *observed);
    }

    type CalibRow = (String, f64, f64, [(f64, f64); 6]);

    // Build rows with expected points
    let mut rows: Vec<CalibRow> = by_team
        .into_iter()
        .map(|(name, rounds)| {
            let tgt: f64 = (0..6).map(|i| rounds[i].0 * WEIGHTS[i]).sum();
            let cal: f64 = (0..6).map(|i| rounds[i].1 * WEIGHTS[i]).sum();
            (name, tgt, cal, rounds)
        })
        .collect();
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let nw = rows.iter().map(|r| r.0.len()).max().unwrap_or(4).max(4);
    let sep = "\u{2501}".repeat(GW);
    let pad = "\u{2501}".repeat(nw + 2);

    // Header line 1: column group names
    println!();
    print!("{:nw$}  ", "");
    print!("\u{2503}{:^GW$}", "PTS");
    for c in &COLS {
        print!("\u{2503}{:^GW$}", c);
    }
    println!();

    // Header line 2: Tgt / Cal sub-headers
    print!("{:<nw$}  ", "Team");
    for _ in 0..7 {
        print!("\u{2503}  Tgt   Cal  ");
    }
    println!();

    // Separator
    print!("{pad}");
    for _ in 0..7 {
        print!("\u{254B}{sep}");
    }
    println!();

    // Data rows
    for (name, tgt_pts, cal_pts, rounds) in &rows {
        print!("{:<nw$}  ", name);
        print!("\u{2503} {:5.1} {:5.1} ", tgt_pts, cal_pts);
        for &(t, c) in rounds {
            print!("\u{2503} {:5.1} {:5.1} ", t * 100.0, c * 100.0);
        }
        println!();
    }
}
