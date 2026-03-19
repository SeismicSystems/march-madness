//! Seed Redis with fake bracket data for local development.
//!
//! Generates random entries, a mid-tournament status, sample groups,
//! and sample mirrors so the leaderboard, groups, and mirrors UI have
//! data to display without needing a running chain or real indexing.

use crate::redis_store;
use eyre::{Result, bail};
use rand::Rng;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::*;
use seismic_march_madness::types::{GameScore, GameState, GameStatus, TournamentStatus};
use tracing::info;

/// Tag names to randomly assign to some entries.
const TAGS: &[&str] = &[
    "Duke4Lyfe",
    "BracketBuster",
    "MarchMadness",
    "CinderellaStory",
    "FinalFourOrBust",
    "ChalkPicker",
    "UpsetKing",
    "BuzzerBeater",
    "DunkCity",
    "BigDanceEnergy",
    "NetCutters",
    "SweetSixteen",
    "CourtVision",
    "FullCourtPress",
    "NothingButNet",
];

/// Group definitions: (slug, display_name, member_count, entry_fee_wei).
const GROUPS: &[(&str, &str, usize, &str)] = &[
    ("office-pool", "Office Pool", 10, "100000000000000000"), // 0.1 ETH
    ("crypto-degens", "Crypto Degens", 15, "500000000000000000"), // 0.5 ETH
    ("family", "Family Bracket", 5, "0"),                     // Free
];

/// Mirror definitions: (slug, display_name, entry_count).
const MIRRORS: &[(&str, &str, usize)] = &[
    ("mens-league", "Men's League", 6),
    ("womens-league", "Women's League", 5),
];

/// How many of the 63 games should be "final" in the seed scenario.
const FINAL_GAMES: usize = 24;
/// How many games should be "live".
const LIVE_GAMES: usize = 3;

pub async fn run(redis: &mut MultiplexedConnection, entries: usize, clean: bool) -> Result<()> {
    if std::env::var("DANGEROUSLY_SEED_REDIS").as_deref() != Ok("1") {
        bail!(
            "seed command requires DANGEROUSLY_SEED_REDIS=1 env var.\n\
             This is a safety guard — never set this on production machines."
        );
    }

    if clean {
        info!("cleaning existing data");
        clean_redis(redis).await?;
    }

    info!(entries, "seeding entries");
    let addresses = seed_entries(redis, entries).await?;

    info!("seeding tournament status");
    seed_tournament_status(redis).await?;

    info!("seeding groups");
    seed_groups(redis, &addresses).await?;

    info!("seeding mirrors");
    seed_mirrors(redis).await?;

    info!(
        entries,
        groups = GROUPS.len(),
        mirrors = MIRRORS.len(),
        "seed complete — run the forecaster then start the server:\n\
         cargo run -p march-madness-forecaster\n\
         cargo run -p march-madness-server"
    );
    Ok(())
}

/// Delete only the Redis keys that the seed command writes.
async fn clean_redis(redis: &mut MultiplexedConnection) -> Result<()> {
    let keys: &[&str] = &[
        KEY_ENTRIES,
        KEY_GROUPS,
        KEY_GROUP_MEMBERS,
        KEY_GROUP_SLUGS,
        KEY_ADDRESS_GROUPS,
        KEY_GAMES,
        KEY_FORECASTS,
        KEY_TEAM_PROBS,
        KEY_MIRRORS,
        KEY_MIRROR_SLUGS,
        KEY_MIRROR_ENTRIES,
    ];
    let deleted: usize = redis::cmd("DEL").arg(keys).query_async(redis).await?;
    info!(deleted, "cleaned seed-related Redis keys");
    Ok(())
}

/// Generate N fake entries with random brackets and optional tags.
async fn seed_entries(redis: &mut MultiplexedConnection, count: usize) -> Result<Vec<String>> {
    let mut rng = rand::rng();
    let mut addresses = Vec::with_capacity(count);

    for i in 1..=count {
        let address = format!("0x{:040x}", i);

        // Random 63-bit bracket with sentinel bit set.
        let raw: u64 = rng.random::<u64>();
        let bracket = (raw & 0x7FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
        let bracket_hex = format!("0x{:016x}", bracket);

        let block = rng.random_range(100..10_000u64);
        let timestamp = 1_700_000_000 + rng.random_range(0..1_000_000u64);

        redis_store::upsert_bracket_submitted(redis, &address, block, timestamp).await?;
        redis_store::set_bracket(redis, &address, &bracket_hex).await?;

        // ~60% of entries get a tag.
        if rng.random_range(0..10u32) < 6 {
            let tag = TAGS[rng.random_range(0..TAGS.len())];
            // Append a number to make tags more unique.
            let tag = format!("{tag}{i}");
            redis_store::update_tag(redis, &address, &tag).await?;
        }

        addresses.push(address);
    }

    info!(count, "entries seeded");
    Ok(addresses)
}

/// Write a mid-tournament TournamentStatus to Redis, including team reach probabilities.
async fn seed_tournament_status(redis: &mut MultiplexedConnection) -> Result<()> {
    let mut rng = rand::rng();
    let mut games = Vec::with_capacity(63);

    for i in 0..63u8 {
        let game = if (i as usize) < FINAL_GAMES {
            // Final games: random winner.
            let winner = rng.random::<bool>();
            let (s1, s2) = if winner {
                (rng.random_range(60..95u32), rng.random_range(45..85u32))
            } else {
                (rng.random_range(45..85u32), rng.random_range(60..95u32))
            };
            GameStatus {
                game_index: i,
                status: GameState::Final,
                score: Some(GameScore {
                    team1: s1,
                    team2: s2,
                }),
                winner: Some(winner),
                team1_win_probability: None,
                seconds_remaining: None,
                period: None,
                ncaa_game_id: None,
            }
        } else if (i as usize) < FINAL_GAMES + LIVE_GAMES {
            // Live games: in-progress scores.
            let s1 = rng.random_range(20..55u32);
            let s2 = rng.random_range(20..55u32);
            let secs = rng.random_range(60..1200i32);
            let period = if rng.random::<bool>() { 1 } else { 2 };
            GameStatus {
                game_index: i,
                status: GameState::Live,
                score: Some(GameScore {
                    team1: s1,
                    team2: s2,
                }),
                winner: None,
                team1_win_probability: Some(rng.random_range(0.2..0.8f64)),
                seconds_remaining: Some(secs),
                period: Some(period),
                ncaa_game_id: Some(rng.random_range(100..999i64)),
            }
        } else {
            GameStatus::upcoming(i)
        };
        games.push(game);
    }

    let status = TournamentStatus {
        games,
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    };

    let json = serde_json::to_string(&status)?;
    let () = redis.set(KEY_GAMES, &json).await?;
    info!(
        final_games = FINAL_GAMES,
        live_games = LIVE_GAMES,
        "tournament status seeded"
    );
    Ok(())
}

/// Create sample groups and assign members from the entry pool.
async fn seed_groups(redis: &mut MultiplexedConnection, addresses: &[String]) -> Result<()> {
    for (group_id, &(slug, display_name, member_count, entry_fee)) in GROUPS.iter().enumerate() {
        let group_id = group_id as u32 + 1;
        let creator = addresses.first().map(|a| a.as_str()).unwrap_or("0x0");

        redis_store::create_group(
            redis,
            group_id,
            slug,
            display_name,
            creator,
            false,
            entry_fee,
        )
        .await?;

        // Add members from the address pool (wrapping if needed).
        let members_to_add = member_count.min(addresses.len());
        for i in 0..members_to_add {
            let addr = &addresses[i % addresses.len()];
            redis_store::member_joined(redis, group_id, addr).await?;
        }

        info!(slug, members = members_to_add, "group seeded");
    }

    Ok(())
}

/// Create sample mirrors with random bracket entries.
async fn seed_mirrors(redis: &mut MultiplexedConnection) -> Result<()> {
    let mut rng = rand::rng();

    for (mirror_id, &(slug, display_name, entry_count)) in MIRRORS.iter().enumerate() {
        let mirror_id = (mirror_id + 1) as u64;
        let admin = "0x0000000000000000000000000000000000000001";

        redis_store::create_mirror(redis, mirror_id, slug, display_name, admin).await?;

        for i in 1..=entry_count {
            let entry_slug = format!("{slug}-entry-{i}");
            let raw: u64 = rng.random::<u64>();
            let bracket = (raw & 0x7FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
            let bracket_hex = format!("0x{:016x}", bracket);
            redis_store::mirror_entry_added(redis, mirror_id, &entry_slug, &bracket_hex).await?;
        }

        info!(slug, entries = entry_count, "mirror seeded");
    }

    Ok(())
}
