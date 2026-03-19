//! Yahoo Fantasy Sports API types and HTTP client.

use eyre::{bail, eyre};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::cache;

// ── API base URL ──────────────────────────────────────────────────────

const YAHOO_API_BASE: &str = "https://pylon.sports.yahoo.com/v1/gql/call/tourney";

const GAME_KEY: &str = "467";

// ── Bracket endpoint types ────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct BracketResponse {
    pub data: BracketData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketData {
    pub fantasy_game: FantasyGame,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FantasyGame {
    pub tournament: Tournament,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tournament {
    pub slots: Vec<Slot>,
    pub tournament_teams: Vec<TournamentTeam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub slot_id: String,
    pub region_id: String,
    pub round_id: String,
    #[serde(default)]
    pub previous_slot_ids: Vec<String>,
    #[serde(default)]
    pub editorial_game: Option<EditorialGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialGame {
    pub bracket_top_team: SlotTeam,
    pub bracket_bottom_team: SlotTeam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotTeam {
    pub editorial_team_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTeam {
    pub editorial_team_key: String,
    pub editorial_team: EditorialTeamInfo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialTeamInfo {
    pub display_name: String,
}

// ── Group members endpoint types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMembersResponse {
    pub data: GroupMembersData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMembersData {
    pub fantasy_teams: Vec<FantasyTeamSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FantasyTeamSummary {
    pub fantasy_team_id: String,
    pub name: String,
    pub user: YahooUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YahooUser {
    pub display_name: String,
}

// ── Team picks endpoint types ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamPicksResponse {
    pub data: TeamPicksData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPicksData {
    pub bracket_picks: BracketPicks,
    pub fantasy_team: FantasyTeamInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BracketPicks {
    pub picks: Vec<Pick>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pick {
    pub slot_id: String,
    pub selected_team_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FantasyTeamInfo {
    pub name: String,
}

// ── HTTP client ───────────────────────────────────────────────────────

pub struct YahooClient {
    client: Client,
    cookie: Option<String>,
}

impl YahooClient {
    pub fn new() -> eyre::Result<Self> {
        let cookie = std::env::var("YAHOO_COOKIE").ok();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { client, cookie })
    }

    /// Fetch the tournament bracket structure (public, no auth).
    pub fn fetch_bracket(&self, force_refresh: bool) -> eyre::Result<BracketResponse> {
        if !force_refresh
            && let Some(cached) = cache::load::<BracketResponse>("bracket", cache::TTL_BRACKET)
        {
            info!("using cached bracket data");
            return Ok(cached);
        }

        let url = format!(
            "{}/bracket?gameKey={}&yspRemoveNulls=true&ysp_src=tdv2-app-fantasy",
            YAHOO_API_BASE, GAME_KEY
        );
        debug!("fetching bracket: {}", url);

        let resp = self.client.get(&url).send()?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("Yahoo bracket API error ({}): {}", status, body);
        }

        let data: BracketResponse = resp.json()?;
        cache::save("bracket", &data)?;
        info!(
            "fetched bracket: {} slots, {} teams",
            data.data.fantasy_game.tournament.slots.len(),
            data.data.fantasy_game.tournament.tournament_teams.len()
        );
        Ok(data)
    }

    /// Fetch group members (requires YAHOO_COOKIE auth). Paginates automatically.
    pub fn fetch_group_members(
        &self,
        group_id: u32,
        force_refresh: bool,
    ) -> eyre::Result<Vec<FantasyTeamSummary>> {
        let cache_key = format!("groups/{}/members", group_id);
        if !force_refresh
            && let Some(cached) = cache::load::<GroupMembersData>(&cache_key, cache::TTL_MEMBERS)
        {
            info!(
                "using cached group members ({} entries)",
                cached.fantasy_teams.len()
            );
            return Ok(cached.fantasy_teams);
        }

        let cookie = self
            .cookie
            .as_ref()
            .ok_or_else(|| eyre!("YAHOO_COOKIE env var required for group members endpoint"))?;

        let mut all_teams = Vec::new();
        let mut start = 0u32;
        let limit = 50u32;

        loop {
            let url = format!(
                "{}/groupMembers?gameKey={}&yspRemoveNulls=true&ysp_src=tdv2-app-fantasy&groupId={}&start={}&limit={}",
                YAHOO_API_BASE, GAME_KEY, group_id, start, limit
            );
            debug!("fetching group members page (start={}): {}", start, url);

            let resp = self
                .client
                .get(&url)
                .header("Cookie", cookie.as_str())
                .send()?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().unwrap_or_default();
                bail!("Yahoo group members API error ({}): {}", status, body);
            }

            let page: GroupMembersResponse = resp.json()?;
            let count = page.data.fantasy_teams.len();
            all_teams.extend(page.data.fantasy_teams);
            debug!(
                "page start={}: got {} members (total: {})",
                start,
                count,
                all_teams.len()
            );

            if count < limit as usize {
                break;
            }
            start += limit;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        info!("fetched {} group members", all_teams.len());

        // Cache the aggregated result
        let data = GroupMembersData {
            fantasy_teams: all_teams.clone(),
        };
        cache::save(&cache_key, &data)?;

        Ok(all_teams)
    }

    /// Fetch a team's bracket picks (public after bracket lock).
    pub fn fetch_team_picks(
        &self,
        team_id: &str,
        group_id: u32,
        force_refresh: bool,
    ) -> eyre::Result<TeamPicksResponse> {
        let cache_key = format!("groups/{}/entries/{}", group_id, team_id);
        if !force_refresh
            && let Some(cached) = cache::load::<TeamPicksResponse>(&cache_key, cache::TTL_PICKS)
        {
            debug!("using cached picks for team {}", team_id);
            return Ok(cached);
        }

        let url = format!(
            "{}/teamPicks?gameKey={}&yspRemoveNulls=true&ysp_src=tdv2-app-fantasy&teamId={}",
            YAHOO_API_BASE, GAME_KEY, team_id
        );
        debug!("fetching picks for team {}: {}", team_id, url);

        let resp = self.client.get(&url).send()?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!(
                "Yahoo team picks API error ({}) for team {}: {}",
                status,
                team_id,
                body
            );
        }

        let data: TeamPicksResponse = resp.json()?;
        cache::save(&cache_key, &data)?;
        Ok(data)
    }
}
