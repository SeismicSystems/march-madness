//! Scoreboard endpoint — fetch live/final/upcoming game data.

use tracing::debug;

use crate::NcaaApiError;
use crate::client::{NCAA_API_BASE, NcaaClient};
use crate::types::{Contest, ScoreboardGqlResponse, SportCode};

/// Persisted query hash for the scoreboard endpoint.
const SCOREBOARD_HASH: &str = "7287cda610a9326931931080cb3a604828febe6fe3c9016a7e4a36db99efdb7c";

/// Compute NCAA season year from a date.
/// Convention: months Jan-Jun → year - 1, Jul-Dec → year.
pub fn season_year(year: i32, month: u32) -> i32 {
    if month < 7 { year - 1 } else { year }
}

/// Build the scoreboard URL for a given sport, date, and season year.
fn build_scoreboard_url(sport: SportCode, date: &str, season_year: i32) -> String {
    let variables = serde_json::json!({
        "sportCode": sport.as_str(),
        "division": 1,
        "seasonYear": season_year,
        "contestDate": date
    });
    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": SCOREBOARD_HASH
        }
    });
    format!(
        "{}?extensions={}&variables={}",
        NCAA_API_BASE,
        urlencoded(&extensions.to_string()),
        urlencoded(&variables.to_string())
    )
}

/// Fetch the scoreboard for a given date.
///
/// `date` should be in "YYYY/MM/DD" format (e.g. "2026/03/15").
pub async fn fetch_scoreboard(
    client: &NcaaClient,
    sport: SportCode,
    date: &str,
) -> Result<Vec<Contest>, NcaaApiError> {
    // Parse year/month from date string
    let parts: Vec<&str> = date.split('/').collect();
    if parts.len() != 3 {
        return Err(NcaaApiError::Config(format!(
            "invalid date format: {date} (expected YYYY/MM/DD)"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| NcaaApiError::Config(format!("invalid year in date: {date}")))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| NcaaApiError::Config(format!("invalid month in date: {date}")))?;

    let sy = season_year(year, month);
    let url = build_scoreboard_url(sport, date, sy);

    debug!("fetching scoreboard for {sport} on {date} (season {sy})");
    let body = client.get(&url).await?;

    let gql: ScoreboardGqlResponse =
        serde_json::from_str(&body).map_err(|e| NcaaApiError::Parse(e.to_string()))?;

    let contests = gql
        .data
        .and_then(|d| d.scoreboard)
        .unwrap_or_default()
        .into_iter()
        .map(Contest::from)
        .collect();

    Ok(contests)
}

/// URL-encode a string (minimal: just what's needed for query params).
fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_season_year() {
        assert_eq!(season_year(2026, 3), 2025); // March 2026 → 2025 season
        assert_eq!(season_year(2026, 11), 2026); // Nov 2026 → 2026 season
        assert_eq!(season_year(2025, 6), 2024); // June 2025 → 2024 season
        assert_eq!(season_year(2025, 7), 2025); // July 2025 → 2025 season
    }

    #[test]
    fn test_build_scoreboard_url() {
        let url = build_scoreboard_url(SportCode::Mbb, "2026/03/15", 2025);
        assert!(url.starts_with(NCAA_API_BASE));
        assert!(url.contains("MBB"));
        assert!(url.contains("2025")); // season year
        assert!(url.contains(SCOREBOARD_HASH));
    }

    #[test]
    fn test_parse_scoreboard_response() {
        let json = r#"{
            "data": {
                "scoreboard": [
                    {
                        "contestId": 12345,
                        "teams": [
                            {"nameShort": "Duke", "score": "82", "seed": "1", "isWinner": true, "isHome": false},
                            {"nameShort": "Washington", "score": "55", "seed": "16", "isWinner": false, "isHome": true}
                        ],
                        "gameState": "F",
                        "currentPeriod": "FINAL",
                        "contestClock": "0:00",
                        "startTimeEpoch": "1742000000",
                        "startDate": "2026-03-15",
                        "startTime": "12:00PM ET",
                        "finalMessage": "FINAL"
                    },
                    {
                        "contestId": 12346,
                        "teams": [
                            {"nameShort": "Michigan", "score": "45", "seed": "1", "isWinner": false, "isHome": true},
                            {"nameShort": "Northwestern", "score": "38", "seed": "16", "isWinner": false, "isHome": false}
                        ],
                        "gameState": "I",
                        "currentPeriod": "2",
                        "contestClock": "8:30",
                        "startTimeEpoch": "1742000000",
                        "startDate": "2026-03-15",
                        "startTime": "1:00PM ET",
                        "finalMessage": ""
                    }
                ]
            }
        }"#;

        let gql: ScoreboardGqlResponse = serde_json::from_str(json).unwrap();
        let contests: Vec<Contest> = gql
            .data
            .unwrap()
            .scoreboard
            .unwrap()
            .into_iter()
            .map(Contest::from)
            .collect();

        assert_eq!(contests.len(), 2);

        // Final game
        assert!(contests[0].is_final());
        assert_eq!(contests[0].scores(), Some((82, 55)));
        assert_eq!(contests[0].teams[0].name_short, "Duke");

        // Live game
        assert!(contests[1].is_live());
        assert_eq!(contests[1].scores(), Some((45, 38)));
        assert_eq!(contests[1].clock_seconds(), Some(8 * 60 + 30));
        assert_eq!(contests[1].period_number(), Some(2));
    }
}
