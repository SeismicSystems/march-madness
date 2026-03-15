use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use tracing::warn;

use crate::auth::workspace_root;
use crate::types::Market;

#[derive(Debug, Deserialize)]
struct TeamNamesConfig {
    names: HashMap<String, String>,
}

pub fn load_team_name_map() -> HashMap<String, String> {
    let toml_path = workspace_root()
        .join("crates")
        .join("kalshi")
        .join("team_names.toml");
    match fs::read_to_string(&toml_path) {
        Ok(content) => {
            let config: TeamNamesConfig = toml::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", toml_path.display(), e));
            config.names
        }
        Err(_) => {
            warn!("team_names.toml not found, using raw Kalshi names");
            HashMap::new()
        }
    }
}

pub fn extract_team_name(market: &Market) -> String {
    if let Some(ref yst) = market.yes_sub_title {
        let yst = yst.trim();
        if !yst.is_empty() && yst.to_lowercase() != "yes" {
            return yst.to_string();
        }
    }
    let title = &market.title;
    if let Some(rest) = title.strip_prefix("Will ") {
        for sep in &[" win ", " qualify ", " make ", " reach ", " advance "] {
            if let Some(idx) = rest.to_lowercase().find(sep) {
                return rest[..idx].trim().to_string();
            }
        }
    }
    title.clone()
}
