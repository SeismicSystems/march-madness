//! Redis writer for tournament status.

use eyre::{Context, Result};
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::KEY_GAMES;
use tracing::debug;

/// Write tournament status JSON to Redis.
pub async fn write_tournament_status(
    conn: &mut MultiplexedConnection,
    status: &seismic_march_madness::TournamentStatus,
) -> Result<()> {
    let json = serde_json::to_string(status).wrap_err("failed to serialize tournament status")?;
    conn.set::<_, _, ()>(KEY_GAMES, &json)
        .await
        .wrap_err("failed to write tournament status to Redis")?;
    debug!("wrote tournament status to Redis");
    Ok(())
}

/// Read tournament status from Redis. Returns None if key doesn't exist.
pub async fn read_tournament_status(
    conn: &mut MultiplexedConnection,
) -> Result<Option<seismic_march_madness::TournamentStatus>> {
    let json: Option<String> = conn
        .get(KEY_GAMES)
        .await
        .wrap_err("failed to read tournament status from Redis")?;
    match json {
        Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}
