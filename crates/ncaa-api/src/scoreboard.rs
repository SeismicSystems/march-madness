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

    let raw_contests = gql.data.and_then(|d| d.contests).unwrap_or_default();

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
    fn test_parse_scoreboard_response_current_format() {
        // Current NCAA API format: "contests" key, numeric score/seed/epoch
        let json = r#"{
            "data": {
                "contests": [
                    {
                        "contestId": 6534597,
                        "teams": [
                            {"nameShort": "Ohio St.", "score": 28, "seed": 8, "isWinner": false, "isHome": true, "seoname": "ohio-st"},
                            {"nameShort": "TCU", "score": 41, "seed": 9, "isWinner": false, "isHome": false, "seoname": "tcu"}
                        ],
                        "gameState": "I",
                        "currentPeriod": "2nd",
                        "contestClock": "15:56",
                        "startTimeEpoch": 1773936900,
                        "startDate": "03/19/2026",
                        "startTime": "12:15",
                        "finalMessage": "2ND HALF"
                    },
                    {
                        "contestId": 6534598,
                        "teams": [
                            {"nameShort": "Duke", "score": 82, "seed": 1, "isWinner": true, "isHome": false, "seoname": "duke"},
                            {"nameShort": "Siena", "score": 55, "seed": 16, "isWinner": false, "isHome": true, "seoname": "siena"}
                        ],
                        "gameState": "F",
                        "currentPeriod": "FINAL",
                        "contestClock": "0:00",
                        "startTimeEpoch": 1773936900,
                        "startDate": "03/19/2026",
                        "startTime": "12:15",
                        "finalMessage": "FINAL"
                    }
                ]
            }
        }"#;

        let gql: ScoreboardGqlResponse = serde_json::from_str(json).unwrap();
        let contests: Vec<Contest> = gql
            .data
            .unwrap()
            .contests
            .unwrap()
            .into_iter()
            .map(|r| Contest::try_from(r).unwrap())
            .collect();

        assert_eq!(contests.len(), 2);

        // Live game
        assert!(contests[0].is_live());
        assert_eq!(contests[0].scores(), Some((28, 41)));
        assert_eq!(contests[0].teams[0].name_short, "Ohio St.");
        assert_eq!(contests[0].teams[0].seed, Some(8));
        match &contests[0].state {
            crate::ContestState::Live {
                period,
                clock_seconds,
            } => {
                assert_eq!(*clock_seconds, Some(15 * 60 + 56));
                assert_eq!(period.unwrap().as_number(), 2);
            }
            _ => panic!("expected Live state"),
        }

        // Final game
        assert!(contests[1].is_final());
        assert_eq!(contests[1].scores(), Some((82, 55)));
        assert_eq!(contests[1].teams[0].name_short, "Duke");
    }

    #[test]
    fn test_parse_scoreboard_response_legacy_format() {
        // Legacy format: "scoreboard" key, string score/seed/epoch
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
                    }
                ]
            }
        }"#;

        let gql: ScoreboardGqlResponse = serde_json::from_str(json).unwrap();
        let contests: Vec<Contest> = gql
            .data
            .unwrap()
            .contests
            .unwrap()
            .into_iter()
            .map(|r| Contest::try_from(r).unwrap())
            .collect();

        assert_eq!(contests.len(), 1);
        assert!(contests[0].is_final());
        assert_eq!(contests[0].scores(), Some((82, 55)));
        assert_eq!(contests[0].start_time_epoch, Some(1742000000));
    }
}
