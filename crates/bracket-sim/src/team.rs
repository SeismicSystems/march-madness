use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::path::Path;

use crate::{MAX_PACE, MAX_RTG, MIN_PACE, MIN_RTG, UPDATE_FACTOR, metrics::Metrics};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Team {
    pub team: String,
    pub seed: u8,
    pub region: String,
    #[serde(flatten)]
    pub metrics: Metrics,
    #[serde(default)]
    pub goose: f64,
}

/// Row from the bracket CSV (team, seed, region).
#[derive(Debug, Deserialize)]
struct BracketRow {
    team: String,
    seed: u8,
    region: String,
}

/// Row from the KenPom CSV (team, ortg, drtg, pace[, goose]).
#[derive(Debug, Deserialize)]
struct KenpomRow {
    team: String,
    ortg: f64,
    drtg: f64,
    pace: f64,
    #[serde(default)]
    goose: f64,
}

/// Tournament JSON format (data/{year}/tournament.json).
#[derive(Debug, Deserialize)]
struct TournamentJson {
    teams: Vec<TournamentJsonTeam>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentJsonTeam {
    /// Null for First Four slots.
    #[serde(default)]
    name: Option<String>,
    seed: u8,
    region: String,
    /// Present when this slot is decided by a First Four game.
    #[serde(default)]
    first_four: Option<FirstFourEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirstFourEntry {
    teams: Vec<FirstFourTeam>,
    winner: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirstFourTeam {
    name: String,
}

impl TournamentJsonTeam {
    /// Resolved display name: the team name, or "A/B" combo for FF slots.
    fn display_name(&self) -> String {
        if let Some(ref name) = self.name {
            return name.clone();
        }
        if let Some(ref ff) = self.first_four
            && ff.teams.len() == 2
        {
            return format!("{}/{}", ff.teams[0].name, ff.teams[1].name);
        }
        String::from("TBD")
    }
}

/// A bracket entry (name, seed, region) before joining with KenPom ratings.
struct BracketEntry {
    name: String,
    seed: u8,
    region: String,
}

fn load_kenpom_map(kenpom_path: &str) -> io::Result<HashMap<String, (Metrics, f64)>> {
    let kenpom_file = File::open(kenpom_path)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", kenpom_path, e)))?;
    load_kenpom_map_from_reader(kenpom_file)
}

fn load_kenpom_map_from_str(csv_content: &str) -> io::Result<HashMap<String, (Metrics, f64)>> {
    load_kenpom_map_from_reader(csv_content.as_bytes())
}

fn load_kenpom_map_from_reader<R: io::Read>(
    reader: R,
) -> io::Result<HashMap<String, (Metrics, f64)>> {
    let mut kenpom_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    let mut kenpom_map: HashMap<String, (Metrics, f64)> = HashMap::new();
    for row in kenpom_reader.deserialize() {
        let r: KenpomRow = row?;
        kenpom_map.insert(
            r.team,
            (
                Metrics {
                    ortg: r.ortg,
                    drtg: r.drtg,
                    pace: r.pace,
                },
                r.goose,
            ),
        );
    }
    Ok(kenpom_map)
}

/// Join bracket entries with KenPom ratings, validate structure, and return teams.
fn join_with_kenpom(entries: Vec<BracketEntry>, kenpom_path: &str) -> io::Result<Vec<Team>> {
    let kenpom_map = load_kenpom_map(kenpom_path)?;

    let mut teams = Vec::new();
    let mut missing = Vec::new();

    for entry in entries {
        match kenpom_map.get(&entry.name) {
            Some((metrics, goose)) => {
                teams.push(Team {
                    team: entry.name,
                    seed: entry.seed,
                    region: entry.region,
                    metrics: *metrics,
                    goose: *goose,
                });
            }
            None => {
                missing.push(entry.name);
            }
        }
    }

    if !missing.is_empty() {
        panic!(
            "FATAL: {} bracket team(s) not found in KenPom file '{}':\n  {}\n\
             Fix the team name mapping or update the KenPom data.",
            missing.len(),
            kenpom_path,
            missing.join("\n  ")
        );
    }

    validate_bracket_structure(&teams);
    Ok(teams)
}

/// Load teams by joining a tournament JSON (data/{year}/tournament.json) with a KenPom CSV.
/// The JSON provides bracket structure (name, seed, region); KenPom provides ratings.
///
/// For First Four slots (e.g. "Texas/NC State"), looks up both individual teams in KenPom
/// and averages their ratings. The combined entry uses the slash-joined name.
pub fn load_teams_from_json(json_path: &Path, kenpom_path: &str) -> io::Result<Vec<Team>> {
    let json_content = std::fs::read_to_string(json_path)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", json_path.display(), e)))?;
    let tournament: TournamentJson = serde_json::from_str(&json_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {}", json_path.display(), e),
        )
    })?;

    let kenpom_map = load_kenpom_map(kenpom_path)?;
    let mut teams = Vec::new();
    let mut missing = Vec::new();

    for t in tournament.teams {
        let display = t.display_name();
        if let Some(ref ff) = t.first_four {
            // First Four: look up both teams and average their ratings.
            let mut found_metrics = Vec::new();
            let mut found_goose = Vec::new();
            for ff_team in &ff.teams {
                match kenpom_map.get(&ff_team.name) {
                    Some((metrics, goose)) => {
                        found_metrics.push(*metrics);
                        found_goose.push(*goose);
                    }
                    None => missing.push(ff_team.name.clone()),
                }
            }
            if found_metrics.is_empty() {
                // Neither team found — will be reported as missing
                continue;
            }
            let n = found_metrics.len() as f64;
            let avg_metrics = Metrics {
                ortg: found_metrics.iter().map(|m| m.ortg).sum::<f64>() / n,
                drtg: found_metrics.iter().map(|m| m.drtg).sum::<f64>() / n,
                pace: found_metrics.iter().map(|m| m.pace).sum::<f64>() / n,
            };
            let avg_goose = found_goose.iter().sum::<f64>() / n;
            teams.push(Team {
                team: display,
                seed: t.seed,
                region: t.region,
                metrics: avg_metrics,
                goose: avg_goose,
            });
        } else {
            // Normal team: direct lookup.
            match kenpom_map.get(&display) {
                Some((metrics, goose)) => {
                    teams.push(Team {
                        team: display,
                        seed: t.seed,
                        region: t.region,
                        metrics: *metrics,
                        goose: *goose,
                    });
                }
                None => missing.push(display),
            }
        }
    }

    if !missing.is_empty() {
        panic!(
            "FATAL: {} bracket team(s) not found in KenPom file '{}':\n  {}\n\
             Fix the team name mapping or update the KenPom data.",
            missing.len(),
            kenpom_path,
            missing.join("\n  ")
        );
    }

    validate_bracket_structure(&teams);
    Ok(teams)
}

/// Load teams by joining tournament JSON and KenPom CSV from in-memory strings.
///
/// Same logic as [`load_teams_from_json`] but works with embedded data — no filesystem
/// access required.
pub fn load_teams_from_json_str(json_content: &str, kenpom_csv: &str) -> io::Result<Vec<Team>> {
    let tournament: TournamentJson = serde_json::from_str(json_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tournament JSON parse error: {}", e),
        )
    })?;

    let kenpom_map = load_kenpom_map_from_str(kenpom_csv)?;
    let mut teams = Vec::new();
    let mut missing = Vec::new();

    for t in tournament.teams {
        let display = t.display_name();
        if let Some(ref ff) = t.first_four {
            let mut found_metrics = Vec::new();
            let mut found_goose = Vec::new();
            for ff_team in &ff.teams {
                match kenpom_map.get(&ff_team.name) {
                    Some((metrics, goose)) => {
                        found_metrics.push(*metrics);
                        found_goose.push(*goose);
                    }
                    None => missing.push(ff_team.name.clone()),
                }
            }
            if found_metrics.is_empty() {
                continue;
            }
            let n = found_metrics.len() as f64;
            let avg_metrics = Metrics {
                ortg: found_metrics.iter().map(|m| m.ortg).sum::<f64>() / n,
                drtg: found_metrics.iter().map(|m| m.drtg).sum::<f64>() / n,
                pace: found_metrics.iter().map(|m| m.pace).sum::<f64>() / n,
            };
            let avg_goose = found_goose.iter().sum::<f64>() / n;
            teams.push(Team {
                team: display,
                seed: t.seed,
                region: t.region,
                metrics: avg_metrics,
                goose: avg_goose,
            });
        } else {
            match kenpom_map.get(&display) {
                Some((metrics, goose)) => {
                    teams.push(Team {
                        team: display,
                        seed: t.seed,
                        region: t.region,
                        metrics: *metrics,
                        goose: *goose,
                    });
                }
                None => missing.push(display),
            }
        }
    }

    if !missing.is_empty() {
        panic!(
            "FATAL: {} bracket team(s) not found in KenPom data:\n  {}\n\
             Fix the team name mapping or update the KenPom data.",
            missing.len(),
            missing.join("\n  ")
        );
    }

    validate_bracket_structure(&teams);
    Ok(teams)
}

/// Load teams by joining a bracket CSV (team,seed,region) with a KenPom CSV (team,ortg,drtg,pace).
pub fn load_teams(bracket_path: &str, kenpom_path: &str) -> io::Result<Vec<Team>> {
    let bracket_file = File::open(bracket_path)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", bracket_path, e)))?;
    let mut bracket_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bracket_file);

    let mut entries = Vec::new();
    for row in bracket_reader.deserialize() {
        let b: BracketRow = row?;
        entries.push(BracketEntry {
            name: b.team,
            seed: b.seed,
            region: b.region,
        });
    }

    join_with_kenpom(entries, kenpom_path)
}

/// Validate that the loaded teams form a valid 64-team bracket:
/// 4 regions x 16 seeds, unique seeds per region, no duplicate team names.
fn validate_bracket_structure(teams: &[Team]) {
    let mut errors = Vec::new();

    // 1. Exactly 64 teams
    if teams.len() != 64 {
        errors.push(format!("Expected 64 teams, found {}", teams.len()));
    }

    // Check for duplicate team names
    let mut seen_names = HashSet::new();
    for t in teams {
        if !seen_names.insert(&t.team) {
            errors.push(format!("Duplicate team name '{}'", t.team));
        }
    }

    // Group by region
    let mut regions: HashMap<&str, Vec<u8>> = HashMap::new();
    for t in teams {
        regions.entry(&t.region).or_default().push(t.seed);
    }

    // 2. Exactly 4 regions
    if regions.len() != 4 {
        errors.push(format!(
            "Expected 4 regions, found {} ({})",
            regions.len(),
            regions.keys().copied().collect::<Vec<_>>().join(", ")
        ));
    }

    // 3 & 4. Each region: exactly 16 teams, seeds 1-16 each once
    for (region, seeds) in &regions {
        if seeds.len() != 16 {
            errors.push(format!(
                "Region '{}' has {} teams, expected 16",
                region,
                seeds.len()
            ));
        }

        let mut seen_seeds = HashSet::new();
        for &s in seeds {
            if !(1..=16).contains(&s) {
                errors.push(format!("Invalid seed {} in region '{}'", s, region));
            } else if !seen_seeds.insert(s) {
                errors.push(format!("Duplicate seed {} in region '{}'", s, region));
            }
        }

        for expected in 1..=16u8 {
            if !seen_seeds.contains(&expected) && seeds.len() >= 16 {
                errors.push(format!("Missing seed {} in region '{}'", expected, region));
            }
        }
    }

    if !errors.is_empty() {
        panic!(
            "FATAL: Invalid bracket structure:\n  {}\n\
             Ensure exactly 4 regions x 16 seeds (1-16).",
            errors.join("\n  ")
        );
    }
}

/// Load teams from a single combined CSV (legacy format: team,seed,region,ortg,drtg,pace[,goose]).
pub fn load_teams_from_combined_csv(path: &str) -> io::Result<Vec<Team>> {
    let file = File::open(path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut teams = Vec::new();
    for line in reader.deserialize() {
        let team: Team = line?;
        teams.push(team);
    }
    validate_bracket_structure(&teams);
    Ok(teams)
}

/// Load just team names from a bracket CSV (for filtering/validation).
pub fn load_team_names(bracket_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(bracket_path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut names = Vec::new();
    for result in reader.records() {
        let record = result?;
        if let Some(name) = record.get(0) {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// Save teams in KenPom CSV format (team,ortg,drtg,pace,goose), sorted by net rating (best first).
pub fn save_kenpom_csv(teams: &[Team], path: &str) -> io::Result<()> {
    let mut sorted: Vec<&Team> = teams.iter().collect();
    sorted.sort_by(|a, b| {
        let net_a = a.metrics.ortg - a.metrics.drtg;
        let net_b = b.metrics.ortg - b.metrics.drtg;
        net_b
            .partial_cmp(&net_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(["team", "ortg", "drtg", "pace", "goose"])?;
    for team in sorted {
        wtr.write_record([
            &team.team,
            &format!("{:.1}", team.metrics.ortg),
            &format!("{:.1}", team.metrics.drtg),
            &format!("{:.1}", team.metrics.pace),
            &format!("{:.2}", team.goose),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

/// Build a map from individual First Four team names to their slot's display name.
///
/// Uses tournament.json as the source of truth. For example, if a slot has
/// `firstFour: ["Texas", "NC State"]` and `name: "Texas/NC State"`, returns
/// `{"Texas" => "Texas/NC State", "NC State" => "Texas/NC State"}`.
pub fn build_first_four_map_from_json(json_content: &str) -> io::Result<HashMap<String, String>> {
    let tournament: TournamentJson = serde_json::from_str(json_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tournament JSON parse error: {}", e),
        )
    })?;

    let mut ff_map = HashMap::new();
    for t in tournament.teams {
        if let Some(ff) = t.first_four {
            let slot_name = t
                .name
                .unwrap_or_else(|| format!("{}/{}", ff.teams[0].name, ff.teams[1].name));
            for ff_team in ff.teams {
                ff_map.insert(ff_team.name, slot_name.clone());
            }
        }
    }
    Ok(ff_map)
}

/// Build a map from individual First Four team names to their slot's display name,
/// reading tournament.json from a file path.
pub fn build_first_four_map(json_path: &Path) -> io::Result<HashMap<String, String>> {
    let json_content = std::fs::read_to_string(json_path)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", json_path.display(), e)))?;
    build_first_four_map_from_json(&json_content)
}

/// Info about a First Four slot, including which teams and whether a winner is decided.
#[derive(Debug, Clone)]
pub struct FirstFourSlotInfo {
    /// The two individual team names.
    pub teams: [String; 2],
    /// The slot display name (e.g. "Texas/NC State").
    pub slot_name: String,
    /// Region this slot belongs to.
    pub region: String,
    /// The winning team name, if the FF game has been played.
    pub winner: Option<String>,
}

/// Build structured First Four slot info from tournament.json.
pub fn build_first_four_slots_from_json(json_content: &str) -> io::Result<Vec<FirstFourSlotInfo>> {
    let tournament: TournamentJson = serde_json::from_str(json_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tournament JSON parse error: {}", e),
        )
    })?;

    let mut slots = Vec::new();
    for t in tournament.teams {
        if let Some(ff) = t.first_four
            && ff.teams.len() == 2
        {
            let slot_name = t
                .name
                .unwrap_or_else(|| format!("{}/{}", ff.teams[0].name, ff.teams[1].name));
            slots.push(FirstFourSlotInfo {
                teams: [ff.teams[0].name.clone(), ff.teams[1].name.clone()],
                slot_name,
                region: t.region.clone(),
                winner: ff.winner,
            });
        }
    }
    Ok(slots)
}

/// Build structured First Four slot info from a file path.
pub fn build_first_four_slots(json_path: &Path) -> io::Result<Vec<FirstFourSlotInfo>> {
    let json_content = std::fs::read_to_string(json_path)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", json_path.display(), e)))?;
    build_first_four_slots_from_json(&json_content)
}

/// Save calibrated goose values back to a KenPom CSV while preserving individual team metrics.
///
/// After calibration, `teams` contains 64 entries with averaged First Four metrics and
/// slot names (e.g. "Texas/NC State"). This function reads the original KenPom CSV
/// (which has individual rows per team), updates only the goose values, and writes back.
///
/// `ff_to_slot` maps individual First Four team names to their slot name (from tournament.json).
/// For First Four teams, the calibrated goose for the slot is applied to both individual rows.
pub fn save_kenpom_csv_with_goose(
    teams: &[Team],
    original_kenpom_path: &str,
    output_path: &str,
    ff_to_slot: &HashMap<String, String>,
) -> io::Result<()> {
    // 1. Build goose map from calibrated teams (keyed by slot name).
    let goose_map: HashMap<&str, f64> = teams.iter().map(|t| (t.team.as_str(), t.goose)).collect();

    // 2. Read original kenpom CSV rows (preserving individual metrics).
    let original_file = File::open(original_kenpom_path)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", original_kenpom_path, e)))?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(original_file);

    let mut rows: Vec<KenpomRow> = Vec::new();
    for row in reader.deserialize() {
        let r: KenpomRow = row?;
        rows.push(r);
    }

    // 3. Update goose values from calibrated teams.
    //    For FF teams, resolve individual name → slot name → goose.
    for row in &mut rows {
        if let Some(&goose) = goose_map.get(row.team.as_str()) {
            // Direct match (non-FF team).
            row.goose = goose;
        } else if let Some(slot_name) = ff_to_slot.get(&row.team) {
            // FF team: use the slot's calibrated goose.
            if let Some(&goose) = goose_map.get(slot_name.as_str()) {
                row.goose = goose;
            }
        }
    }

    // 4. Sort by net rating (best first) and write.
    rows.sort_by(|a, b| {
        let net_a = a.ortg - a.drtg;
        let net_b = b.ortg - b.drtg;
        net_b
            .partial_cmp(&net_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut wtr = csv::Writer::from_path(output_path)?;
    wtr.write_record(["team", "ortg", "drtg", "pace", "goose"])?;
    for row in &rows {
        wtr.write_record([
            &row.team,
            &format!("{:.1}", row.ortg),
            &format!("{:.1}", row.drtg),
            &format!("{:.1}", row.pace),
            &format!("{:.2}", row.goose),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

impl Team {
    /// Update team metrics based on game performance vs expectations.
    pub fn update_metrics(&mut self, expected: Metrics, observed: Metrics) {
        self.metrics.ortg += (observed.ortg - expected.ortg) * UPDATE_FACTOR;
        self.metrics.drtg += (observed.drtg - expected.drtg) * UPDATE_FACTOR;
        self.metrics.pace += (observed.pace - expected.pace) * UPDATE_FACTOR;

        self.metrics.ortg = self.metrics.ortg.clamp(MIN_RTG, MAX_RTG);
        self.metrics.drtg = self.metrics.drtg.clamp(MIN_RTG, MAX_RTG);
        self.metrics.pace = self.metrics.pace.clamp(MIN_PACE, MAX_PACE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_team(name: &str, seed: u8, region: &str) -> Team {
        Team {
            team: name.to_string(),
            seed,
            region: region.to_string(),
            metrics: Metrics {
                ortg: 110.0,
                drtg: 100.0,
                pace: 70.0,
            },
            goose: 0.0,
        }
    }

    fn make_valid_bracket() -> Vec<Team> {
        let regions = ["East", "West", "South", "Midwest"];
        let mut teams = Vec::new();
        for region in &regions {
            for seed in 1..=16u8 {
                teams.push(make_team(&format!("{}-{}", region, seed), seed, region));
            }
        }
        teams
    }

    #[test]
    fn valid_bracket_passes() {
        validate_bracket_structure(&make_valid_bracket());
    }

    #[test]
    #[should_panic(expected = "Expected 64 teams, found 63")]
    fn missing_team_detected() {
        let mut teams = make_valid_bracket();
        teams.pop();
        validate_bracket_structure(&teams);
    }

    #[test]
    #[should_panic(expected = "Duplicate seed 3 in region")]
    fn duplicate_seed_detected() {
        let mut teams = make_valid_bracket();
        let idx = teams
            .iter()
            .position(|t| t.region == "East" && t.seed == 16)
            .unwrap();
        teams[idx] = make_team("East-dup", 3, "East");
        validate_bracket_structure(&teams);
    }

    #[test]
    #[should_panic(expected = "Duplicate team name")]
    fn duplicate_team_name_detected() {
        let mut teams = make_valid_bracket();
        let r0 = teams[0].region.clone();
        let s0 = teams[0].seed;
        teams[0] = make_team("SameName", s0, &r0);
        let r16 = teams[16].region.clone();
        let s16 = teams[16].seed;
        teams[16] = make_team("SameName", s16, &r16);
        validate_bracket_structure(&teams);
    }

    #[test]
    #[should_panic(expected = "Expected 4 regions, found 3")]
    fn wrong_region_count_detected() {
        let regions = ["East", "West", "South", "Midwest"];
        let mut teams = Vec::new();
        for (i, region) in regions.iter().enumerate() {
            let actual_region = if i == 3 { "East" } else { region };
            for seed in 1..=16u8 {
                teams.push(make_team(
                    &format!("{}-{}", region, seed),
                    seed,
                    actual_region,
                ));
            }
        }
        validate_bracket_structure(&teams);
    }

    #[test]
    fn ff_map_pending_slot() {
        let json = r#"{
            "name": "T", "regions": ["E"],
            "teams": [
                {"name": null, "seed": 16, "region": "E",
                 "firstFour": {"teams": [{"name": "TeamA"}, {"name": "TeamB"}]}}
            ]
        }"#;
        let map = build_first_four_map_from_json(json).unwrap();
        assert_eq!(map.get("TeamA").unwrap(), "TeamA/TeamB");
        assert_eq!(map.get("TeamB").unwrap(), "TeamA/TeamB");
    }

    #[test]
    fn ff_map_decided_slot() {
        let json = r#"{
            "name": "T", "regions": ["E"],
            "teams": [
                {"name": null, "seed": 11, "region": "E",
                 "firstFour": {"teams": [{"name": "X"}, {"name": "Y"}], "winner": "X"}}
            ]
        }"#;
        let map = build_first_four_map_from_json(json).unwrap();
        assert_eq!(map.get("X").unwrap(), "X/Y");
        assert_eq!(map.get("Y").unwrap(), "X/Y");
    }

    #[test]
    fn ff_slots_pending_and_decided() {
        let json = r#"{
            "name": "T", "regions": ["E"],
            "teams": [
                {"name": null, "seed": 16, "region": "E",
                 "firstFour": {"teams": [{"name": "A"}, {"name": "B"}]}},
                {"name": null, "seed": 11, "region": "E",
                 "firstFour": {"teams": [{"name": "X"}, {"name": "Y"}], "winner": "X"}}
            ]
        }"#;
        let slots = build_first_four_slots_from_json(json).unwrap();
        assert_eq!(slots.len(), 2);

        // Pending slot
        assert_eq!(slots[0].teams, ["A", "B"]);
        assert!(slots[0].winner.is_none());
        assert_eq!(slots[0].slot_name, "A/B");
        assert_eq!(slots[0].region, "E");

        // Decided slot
        assert_eq!(slots[1].teams, ["X", "Y"]);
        assert_eq!(slots[1].winner.as_deref(), Some("X"));
        assert_eq!(slots[1].slot_name, "X/Y");
        assert_eq!(slots[1].region, "E");
    }

    #[test]
    fn ff_display_name_pending() {
        let t = TournamentJsonTeam {
            name: None,
            seed: 16,
            region: "E".into(),
            first_four: Some(FirstFourEntry {
                teams: vec![
                    FirstFourTeam { name: "Foo".into() },
                    FirstFourTeam { name: "Bar".into() },
                ],
                winner: None,
            }),
        };
        assert_eq!(t.display_name(), "Foo/Bar");
    }

    #[test]
    fn ff_no_first_four_no_name() {
        let t = TournamentJsonTeam {
            name: None,
            seed: 1,
            region: "E".into(),
            first_four: None,
        };
        assert_eq!(t.display_name(), "TBD");
    }
}
