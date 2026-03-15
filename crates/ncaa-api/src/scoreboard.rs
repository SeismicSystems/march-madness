//! Scoreboard endpoint — fetch live/final/upcoming game data.

use tracing::debug;

use crate::NcaaApiError;
use crate::client::{NcaaClient, build_gql_url};
use crate::types::{Contest, ContestDate, ScoreboardGqlResponse, SportCode};

/// Persisted query hash for the NCAA scoreboard GraphQL endpoint.
/// Shared across all sports (MBB, WBB, etc.) — the sport is specified in the variables.
/// Source: <https://github.com/henrygd/ncaa-api>
const SCOREBOARD_HASH: &str = "7287cda610a9326931931080cb3a604828febe6fe3c9016a7e4a36db99efdb7c";

/// Fetch the scoreboard for a given date.
pub async fn fetch_scoreboard(
    client: &NcaaClient,
    sport: SportCode,
    date: &ContestDate,
) -> Result<Vec<Contest>, NcaaApiError> {
    let sy = date.season_year();
    let variables = serde_json::json!({
        "sportCode": sport.as_str(),
        "division": 1,
        "seasonYear": sy,
        "contestDate": date.as_api_str()
    });
    let url = build_gql_url(SCOREBOARD_HASH, &variables);

    debug!("fetching scoreboard for {sport} on {date} (season {sy})");
    let body = client.get(&url).await?;

    let gql: ScoreboardGqlResponse = serde_json::from_str(&body)?;

    let raw_contests = gql
        .data
        .and_then(|d| d.scoreboard)
        .ok_or_else(|| NcaaApiError::Parse("scoreboard response missing data".into()))?;

    let mut contests = Vec::with_capacity(raw_contests.len());
    for raw in raw_contests {
        contests.push(Contest::try_from(raw)?);
    }

    Ok(contests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::NCAA_API_BASE;

    #[test]
    fn test_contest_date_season_year() {
        // March 2026 → 2025 season (basketball season starts in fall)
        let d = ContestDate::parse("2026/03/15").unwrap();
        assert_eq!(d.season_year(), 2025);

        // November 2026 → 2026 season
        let d2 = ContestDate::parse("2026/11/15").unwrap();
        assert_eq!(d2.season_year(), 2026);
    }

    #[test]
    fn test_build_scoreboard_url() {
        let variables = serde_json::json!({
            "sportCode": "MBB",
            "division": 1,
            "seasonYear": 2025,
            "contestDate": "2026/03/15"
        });
        let url = build_gql_url(SCOREBOARD_HASH, &variables);
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
            .map(|r| Contest::try_from(r).unwrap())
            .collect();

        assert_eq!(contests.len(), 2);

        // Final game
        assert!(contests[0].is_final());
        assert_eq!(contests[0].scores(), Some((82, 55)));
        assert_eq!(contests[0].teams[0].name_short, "Duke");

        // Live game
        assert!(contests[1].is_live());
        assert_eq!(contests[1].scores(), Some((45, 38)));
        assert_eq!(contests[1].clock_seconds, Some(8 * 60 + 30));
        assert_eq!(contests[1].period.unwrap().as_number(), 2);
    }
}
