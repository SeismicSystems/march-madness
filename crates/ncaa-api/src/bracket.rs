//! Bracket endpoint — fetch tournament bracket structure with regions, seeds, and matchups.

use tracing::debug;

use crate::NcaaApiError;
use crate::client::{NCAA_API_BASE, NcaaClient};

/// Persisted query hash for the NCAA bracket/championship GraphQL endpoint.
/// Source: <https://github.com/henrygd/ncaa-api>
const BRACKET_HASH: &str = "e651c2602fb9e82cdad6e947389600c6b69e0e463e437b78bf7ec614d6d15f80";

/// Fetch the tournament bracket for a given sport, division, and year.
///
/// Returns the full championship bracket including regions, games, and teams.
///
/// # Arguments
/// * `sport_url` — NCAA sport URL slug (e.g. `"basketball-men"`, `"basketball-women"`)
/// * `division` — NCAA division number (1 for D1)
/// * `year` — Tournament year (e.g. 2026)
pub async fn fetch_bracket(
    client: &NcaaClient,
    sport_url: &str,
    division: u32,
    year: i32,
) -> Result<Championship, NcaaApiError> {
    let variables = serde_json::json!({
        "sportUrl": sport_url,
        "division": division,
        "year": year,
    });
    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": BRACKET_HASH
        }
    });

    let url = format!(
        "{}?operationName=get_championship_ncaa&variables={}&extensions={}",
        NCAA_API_BASE,
        urlencoded(&variables.to_string()),
        urlencoded(&extensions.to_string()),
    );

    debug!("fetching bracket for {sport_url} d{division} {year}");
    let body = client.get(&url).await?;

    let gql: BracketGqlResponse = serde_json::from_str(&body)?;

    let championships = gql
        .data
        .and_then(|d| d.championships)
        .ok_or_else(|| NcaaApiError::Parse("bracket response missing data".into()))?;

    championships
        .into_iter()
        .next()
        .ok_or_else(|| NcaaApiError::Parse("no championships in response".into()))
}

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ── Raw deserialization types ──────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct BracketGqlResponse {
    data: Option<BracketData>,
}

#[derive(Debug, serde::Deserialize)]
struct BracketData {
    championships: Option<Vec<Championship>>,
}

// ── Public types ───────────────────────────────────────────────────

/// A full tournament championship bracket.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Championship {
    pub title: String,
    pub year: i32,
    pub season: i32,
    pub sport_url: String,
    pub games: Vec<BracketGame>,
    pub regions: Vec<BracketRegion>,
}

/// A single game in the bracket.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketGame {
    pub bracket_position_id: u32,
    pub section_id: u32,
    #[serde(default)]
    pub victor_bracket_position_id: Option<u32>,
    pub teams: Vec<BracketTeam>,
    #[serde(default)]
    pub game_state: String,
}

/// A team entry within a bracket game.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketTeam {
    #[serde(default)]
    pub name_short: String,
    #[serde(default)]
    pub name_full: String,
    #[serde(default)]
    pub seed: Option<u32>,
    #[serde(default)]
    pub is_top: bool,
    #[serde(default)]
    pub seoname: String,
    /// Non-null when the team slot is TBA (e.g. awaiting a First Four result).
    #[serde(default)]
    pub text_override: Option<String>,
}

/// A region/section in the bracket.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketRegion {
    pub section_id: u32,
    pub title: String,
    /// Position code: TL (top-left), TR (top-right), BL (bottom-left), BR (bottom-right),
    /// TT (First Four), CC (Final Four/Championship).
    pub region_code: String,
}

impl Championship {
    /// Get games for a specific section (region).
    pub fn games_in_section(&self, section_id: u32) -> Vec<&BracketGame> {
        self.games
            .iter()
            .filter(|g| g.section_id == section_id)
            .collect()
    }

    /// Get R64 (first round) games for a section, sorted by bracket position.
    pub fn r64_games(&self, section_id: u32) -> Vec<&BracketGame> {
        let mut games: Vec<_> = self
            .games
            .iter()
            .filter(|g| g.section_id == section_id && (200..300).contains(&g.bracket_position_id))
            .collect();
        games.sort_by_key(|g| g.bracket_position_id);
        games
    }

    /// Get First Four (play-in) games.
    pub fn first_four_games(&self) -> Vec<&BracketGame> {
        let mut games: Vec<_> = self
            .games
            .iter()
            .filter(|g| (100..200).contains(&g.bracket_position_id))
            .collect();
        games.sort_by_key(|g| g.bracket_position_id);
        games
    }

    /// Get region metadata for a section ID.
    pub fn region_for_section(&self, section_id: u32) -> Option<&BracketRegion> {
        self.regions.iter().find(|r| r.section_id == section_id)
    }

    /// Determine Final Four pairings by tracing `victorBracketPositionId`.
    ///
    /// Returns two pairs of section IDs: `[(a, b), (c, d)]` where `a` plays `b`
    /// and `c` plays `d` in the Final Four.
    pub fn final_four_pairings(&self) -> Result<[(u32, u32); 2], NcaaApiError> {
        // Regional final games are in the 500s range.
        // They feed into F4 semifinal games in the 600s range.
        let regional_finals: Vec<_> = self
            .games
            .iter()
            .filter(|g| (500..600).contains(&g.bracket_position_id))
            .collect();

        // Group by which semifinal they feed into.
        let mut semi_map: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();
        for game in &regional_finals {
            if let Some(victor_bid) = game.victor_bracket_position_id {
                semi_map
                    .entry(victor_bid)
                    .or_default()
                    .push(game.section_id);
            }
        }

        let mut pairings: Vec<(u32, u32)> = Vec::new();
        for sections in semi_map.values() {
            if sections.len() != 2 {
                return Err(NcaaApiError::Parse(format!(
                    "expected 2 regions per semifinal, got {}",
                    sections.len()
                )));
            }
            let (a, b) = (sections[0], sections[1]);
            pairings.push((a.min(b), a.max(b)));
        }

        if pairings.len() != 2 {
            return Err(NcaaApiError::Parse(format!(
                "expected 2 F4 pairings, got {}",
                pairings.len()
            )));
        }

        pairings.sort();
        Ok([pairings[0], pairings[1]])
    }

    /// Build the bracket region order for encoding.
    ///
    /// Returns 4 section IDs ordered so that indices 0,1 play each other in the
    /// Final Four and indices 2,3 play each other. Within each pair, the region
    /// with `regionCode` starting with "T" (top) comes first, then "B" (bottom).
    pub fn bracket_region_order(&self) -> Result<[u32; 4], NcaaApiError> {
        let pairings = self.final_four_pairings()?;

        let mut result = [0u32; 4];
        for (i, (a, b)) in pairings.iter().enumerate() {
            // Put the "top" region (TL/TR) first, "bottom" (BL/BR) second.
            let a_is_top = self
                .region_for_section(*a)
                .map(|r| r.region_code.starts_with('T'))
                .unwrap_or(false);

            if a_is_top {
                result[i * 2] = *a;
                result[i * 2 + 1] = *b;
            } else {
                result[i * 2] = *b;
                result[i * 2 + 1] = *a;
            }
        }

        Ok(result)
    }
}

impl BracketRegion {
    /// Get the cleaned region name (trimmed, title case).
    pub fn name(&self) -> String {
        let trimmed = self.title.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        // Convert "EAST" → "East", "MIDWEST" → "Midwest", etc.
        let mut chars = trimmed.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        first + &chars.as_str().to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_name_cleanup() {
        let r = BracketRegion {
            section_id: 2,
            title: " EAST".into(),
            region_code: "TL".into(),
        };
        assert_eq!(r.name(), "East");

        let r2 = BracketRegion {
            section_id: 5,
            title: " MIDWEST".into(),
            region_code: "BR".into(),
        };
        assert_eq!(r2.name(), "Midwest");
    }

    #[test]
    fn test_urlencoded() {
        let s = r#"{"foo":"bar"}"#;
        let encoded = urlencoded(s);
        assert!(encoded.contains("%22"));
        assert!(!encoded.contains('"'));
    }
}
