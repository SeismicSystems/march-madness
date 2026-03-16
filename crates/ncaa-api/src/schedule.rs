//! Schedule endpoint — determine today's contest date.

use tracing::debug;

use crate::NcaaApiError;
use crate::client::{NcaaClient, build_gql_url};
use crate::types::{ContestDate, ScheduleGqlResponse, SportCode};

/// Persisted query hash for the NCAA schedule GraphQL endpoint.
/// Shared across all sports (MBB, WBB, etc.) — the sport is specified in the variables.
/// Source: <https://github.com/henrygd/ncaa-api>
const SCHEDULE_HASH: &str = "a25ad021179ce1d97fb951a49954dc98da150089f9766e7e85890e439516ffbf";

/// Fetch the schedule for a season.
///
/// Returns dates that have games, as `ContestDate` values.
pub async fn fetch_schedule(
    client: &NcaaClient,
    sport: SportCode,
    season_year: i32,
) -> Result<Vec<ContestDate>, NcaaApiError> {
    let variables = serde_json::json!({
        "sportCode": sport.as_str(),
        "division": 1,
        "seasonYear": season_year
    });
    let url = build_gql_url(SCHEDULE_HASH, &variables);

    debug!("fetching schedule for {sport} season {season_year}");
    let body = client.get(&url).await?;

    let gql: ScheduleGqlResponse = serde_json::from_str(&body)?;

    let entries = gql
        .data
        .and_then(|d| d.schedules)
        .and_then(|s| s.games)
        .ok_or_else(|| NcaaApiError::Parse("schedule response missing data".into()))?;

    let mut dates = Vec::new();
    for entry in entries {
        if entry.count > 0 {
            dates.push(ContestDate::parse(&entry.contest_date)?);
        }
    }

    Ok(dates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schedule_response() {
        let json = r#"{
            "data": {
                "schedules": {
                    "games": [
                        {"contestDate": "03/15/2026", "count": 8},
                        {"contestDate": "03/16/2026", "count": 8},
                        {"contestDate": "03/17/2026", "count": 0}
                    ]
                }
            }
        }"#;

        let gql: ScheduleGqlResponse = serde_json::from_str(json).unwrap();
        let entries = gql.data.unwrap().schedules.unwrap().games.unwrap();
        let dates: Vec<ContestDate> = entries
            .into_iter()
            .filter(|e| e.count > 0)
            .map(|e| ContestDate::parse(&e.contest_date).unwrap())
            .collect();

        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0].as_api_str(), "2026/03/15");
        assert_eq!(dates[1].as_api_str(), "2026/03/16");
    }
}
