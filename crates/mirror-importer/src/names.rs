//! Yahoo team name → NCAA name resolution.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use eyre::eyre;
use serde::Deserialize;
use tracing::{debug, warn};

use seismic_march_madness::{TournamentData, get_teams_in_bracket_order};

#[derive(Debug, Deserialize)]
struct MappingsConfig {
    #[serde(default)]
    yahoo: HashMap<String, String>,
}

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find workspace root")
        .to_path_buf()
}

/// Load the `[yahoo]` section from `data/mappings.toml`.
fn load_yahoo_mappings() -> HashMap<String, String> {
    let path = workspace_root().join("data").join("mappings.toml");
    match fs::read_to_string(&path) {
        Ok(content) => {
            let config: MappingsConfig = toml::from_str(&content)
                .unwrap_or_else(|e| panic!("failed to parse {}: {}", path.display(), e));
            config.yahoo
        }
        Err(_) => {
            warn!("data/mappings.toml not found, using raw Yahoo names");
            HashMap::new()
        }
    }
}

/// Resolver that maps Yahoo editorialTeamKey → bracket position (0-63).
pub struct NameResolver {
    /// editorialTeamKey → bracket position
    key_to_position: HashMap<String, u8>,
    /// editorialTeamKey → resolved NCAA name
    key_to_ncaa_name: HashMap<String, String>,
}

impl NameResolver {
    /// Build the resolver from tournament data and bracket API team list.
    ///
    /// - `tournament`: the embedded tournament data
    /// - `yahoo_teams`: (editorialTeamKey, displayName) pairs from the bracket API
    pub fn new(
        tournament: &TournamentData,
        yahoo_teams: &[(String, String)],
    ) -> eyre::Result<Self> {
        let yahoo_mappings = load_yahoo_mappings();
        let bracket_teams = get_teams_in_bracket_order(tournament);

        // Build NCAA name → position lookup (from 64 bracket-order teams)
        let mut ncaa_to_pos: HashMap<String, u8> = HashMap::new();
        for (i, name) in bracket_teams.iter().enumerate() {
            ncaa_to_pos.insert(name.clone(), i as u8);
        }

        // Also add individual First Four team names (they might appear as picks)
        for team in &tournament.teams {
            if let Some(ref ff) = team.first_four {
                // Find the bracket position of this FF slot (by display_name)
                let display = team.display_name();
                if let Some(&pos) = ncaa_to_pos.get(&display) {
                    for ff_team in &ff.teams {
                        ncaa_to_pos.entry(ff_team.name.clone()).or_insert(pos);
                    }
                }
            }
        }

        debug!("NCAA position map has {} entries", ncaa_to_pos.len());

        // Build editorialTeamKey → NCAA name and position
        let mut key_to_position = HashMap::new();
        let mut key_to_ncaa_name = HashMap::new();

        for (key, yahoo_name) in yahoo_teams {
            // Resolve: Yahoo displayName → mapped name (or identity) → NCAA name
            let ncaa_name = yahoo_mappings
                .get(yahoo_name)
                .cloned()
                .unwrap_or_else(|| yahoo_name.clone());

            if let Some(&pos) = ncaa_to_pos.get(&ncaa_name) {
                key_to_position.insert(key.clone(), pos);
                key_to_ncaa_name.insert(key.clone(), ncaa_name);
            } else {
                // Not in the 64-team bracket (e.g., First Four losers or composite keys).
                // This is expected for teams that lost in the First Four.
                debug!(
                    "team key {} ({} → {}) not in bracket — skipping",
                    key, yahoo_name, ncaa_name
                );
            }
        }

        debug!(
            "resolved {}/{} team keys to bracket positions",
            key_to_position.len(),
            yahoo_teams.len()
        );

        Ok(Self {
            key_to_position,
            key_to_ncaa_name,
        })
    }

    /// Look up a team key's bracket position.
    pub fn position(&self, key: &str) -> eyre::Result<u8> {
        self.key_to_position
            .get(key)
            .copied()
            .ok_or_else(|| eyre!("unknown team key: {}", key))
    }

    /// Look up a team key's NCAA name.
    pub fn ncaa_name(&self, key: &str) -> Option<&str> {
        self.key_to_ncaa_name.get(key).map(|s| s.as_str())
    }
}
