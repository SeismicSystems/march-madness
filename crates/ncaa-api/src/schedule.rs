//! Schedule endpoint — determine today's contest date.

use tracing::debug;

use crate::NcaaApiError;
use crate::client::{NCAA_API_BASE, NcaaClient};
use crate::types::{ScheduleGqlResponse, SportCode};

/// Persisted query hash for the schedule endpoint.
const SCHEDULE_HASH: &str = "a25ad021179ce1d97fb951a49954dc98da150089f9766e7e85890e439516ffbf";

/// Fetch the schedule to find today's contest date.
///
/// Returns dates that have games, in "YYYY/MM/DD" format.
pub async fn fetch_schedule(
    client: &NcaaClient,
    sport: SportCode,
    season_year: i32,
) -> Result<Vec<String>, NcaaApiError> {
    let variables = serde_json::json!({
        "sportCode": sport.as_str(),
        "division": 1,
        "seasonYear": season_year
    });
    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": SCHEDULE_HASH
        }
    });

    let url = format!(
        "{}?extensions={}&variables={}",
        NCAA_API_BASE,
        urlencoded(&extensions.to_string()),
        urlencoded(&variables.to_string())
    );

    debug!("fetching schedule for {sport} season {season_year}");
    let body = client.get(&url).await?;

    let gql: ScheduleGqlResponse =
        serde_json::from_str(&body).map_err(|e| NcaaApiError::Parse(e.to_string()))?;

    let dates = gql
        .data
        .and_then(|d| d.schedule)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.number_of_games > 0)
        .map(|e| e.contest_date)
        .collect();

    Ok(dates)
}

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schedule_response() {
        let json = r#"{
            "data": {
                "schedule": [
                    {"contestDate": "2026/03/15", "numberOfGames": 8},
                    {"contestDate": "2026/03/16", "numberOfGames": 8},
                    {"contestDate": "2026/03/17", "numberOfGames": 0}
                ]
            }
        }"#;

        let gql: ScheduleGqlResponse = serde_json::from_str(json).unwrap();
        let dates: Vec<String> = gql
            .data
            .unwrap()
            .schedule
            .unwrap()
            .into_iter()
            .filter(|e| e.number_of_games > 0)
            .map(|e| e.contest_date)
            .collect();

        assert_eq!(dates, vec!["2026/03/15", "2026/03/16"]);
    }
}
