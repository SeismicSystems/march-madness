//! Sanity check: compare Redis state with on-chain data.
//!
//! Compares entry count and per-group member counts between Redis and the chain.

use crate::provider::IndexerProvider;
use crate::redis_store;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use tracing::{error, info};

pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: &str,
    groups_contract: &str,
) -> Result<()> {
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;
    let groups_addr: Address = groups_contract
        .parse()
        .wrap_err("invalid groups contract address")?;

    // ── Entry count ─────────────────────────────────────────────────
    let on_chain = p.get_entry_count(contract_addr).await?;
    let local = redis_store::get_entry_count(redis).await? as u32;

    info!(local, on_chain, "entry counts");

    if local == on_chain {
        info!("OK — entry counts match");
    } else {
        error!(local, on_chain, "MISMATCH — entry counts differ");
    }

    // ── Group member counts ─────────────────────────────────────────
    let groups = redis_store::get_all_groups(redis).await?;

    if groups.is_empty() {
        info!("no groups in Redis, skipping group check");
        return Ok(());
    }

    info!(
        groups = groups.len(),
        "checking group member counts against chain"
    );

    let mut mismatches = 0u32;
    let mut ok = 0u32;

    for (id_str, data) in &groups {
        let id: u32 = match id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                error!(group_id = %id_str, "invalid group ID in Redis, skipping");
                continue;
            }
        };

        let members = redis_store::get_group_members(redis, id_str).await?;
        let redis_count = members.len() as u32;

        let on_chain_count = match p.get_group_member_count(groups_addr, id).await {
            Ok(c) => c,
            Err(e) => {
                error!(
                    group_id = id,
                    slug = %data.slug,
                    error = %e,
                    "failed to read on-chain member count"
                );
                continue;
            }
        };

        if redis_count != on_chain_count {
            error!(
                group_id = id,
                slug = %data.slug,
                redis = redis_count,
                on_chain = on_chain_count,
                "MISMATCH — group member count"
            );
            mismatches += 1;
        } else {
            ok += 1;
        }
    }

    if mismatches == 0 {
        info!(groups = ok, "OK — all group member counts match chain");
    } else {
        error!(mismatches, ok, total = groups.len(), "group check complete");
    }

    Ok(())
}
