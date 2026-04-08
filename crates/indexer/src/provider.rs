//! Seismic-alloy provider helpers for the indexer.
//!
//! Supports both `SeismicReth` (production / testnet) and `SeismicFoundry` / sanvil
//! (local development) via the `--network` CLI flag.

use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, Bytes, FixedBytes};
use alloy_provider::Provider;
use alloy_rpc_types_eth::{BlockNumberOrTag, Filter, Log};
use alloy_sol_types::{SolCall, SolEvent};
use eyre::{Result, WrapErr, bail};
use seismic_alloy_network::SeismicReth;
use seismic_alloy_network::foundry::SeismicFoundry;
use seismic_alloy_provider::{SeismicProviderBuilder, SeismicUnsignedProvider};
use seismic_alloy_rpc_types::SeismicTransactionRequest;

use crate::contract::{
    BracketSubmitted, BracketUpdated, EntryAdded, EntryRemoved, GroupCreated, MemberJoined,
    MemberLeft, Mirror, MirrorCreated, MirrorEntry, TagSet, getBracketCall, getEntriesCall,
    getEntryBySlugCall, getEntryCountCall, getGroupCall, getMirrorCall, nextMirrorIdCall,
};

/// Network-agnostic indexer provider that wraps either a SeismicReth or SeismicFoundry
/// unsigned provider. Use `--network reth` (default) for production/testnet or
/// `--network foundry` for local development with sanvil.
pub enum IndexerProvider {
    Reth(SeismicUnsignedProvider<SeismicReth>),
    Foundry(SeismicUnsignedProvider<SeismicFoundry>),
}

impl IndexerProvider {
    /// Create a provider for SeismicReth (production / testnet).
    pub fn new_reth(rpc_url: &str) -> Result<Self> {
        let url: reqwest::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
        Ok(Self::Reth(SeismicProviderBuilder::new().connect_http(url)))
    }

    /// Create a provider for SeismicFoundry / sanvil (local development).
    pub fn new_foundry(rpc_url: &str) -> Result<Self> {
        let url: reqwest::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
        Ok(Self::Foundry(
            SeismicProviderBuilder::new().foundry().connect_http(url),
        ))
    }

    /// Get the latest block number.
    pub async fn block_number(&self) -> Result<u64> {
        let num = match self {
            Self::Reth(p) => p.get_block_number().await,
            Self::Foundry(p) => p.get_block_number().await,
        }
        .wrap_err("failed to get block number")?;
        Ok(num)
    }

    /// Get block timestamp by block number.
    pub async fn get_block_timestamp(&self, block_num: u64) -> Result<u64> {
        let tag = BlockNumberOrTag::Number(block_num);
        let timestamp = match self {
            Self::Reth(p) => {
                let block = p
                    .get_block_by_number(tag)
                    .await
                    .wrap_err("failed to get block")?
                    .ok_or_else(|| eyre::eyre!("block {} not found", block_num))?;
                block.header.timestamp
            }
            Self::Foundry(p) => {
                let block = p
                    .get_block_by_number(tag)
                    .await
                    .wrap_err("failed to get block")?
                    .ok_or_else(|| eyre::eyre!("block {} not found", block_num))?;
                block.header.timestamp
            }
        };
        Ok(timestamp)
    }

    // ── Log fetchers (generic by event signature) ────────────────────

    async fn get_logs_for_event(
        &self,
        contract: Address,
        event_sig: FixedBytes<32>,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .address(contract)
            .event_signature(event_sig)
            .from_block(from_block)
            .to_block(to_block);

        let logs = match self {
            Self::Reth(p) => p.get_logs(&filter).await,
            Self::Foundry(p) => p.get_logs(&filter).await,
        }
        .wrap_err("failed to fetch logs")?;
        Ok(logs)
    }

    // ── MarchMadness logs ────────────────────────────────────────────

    pub async fn get_bracket_submitted_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(
            contract,
            BracketSubmitted::SIGNATURE_HASH,
            from_block,
            to_block,
        )
        .await
        .wrap_err("failed to fetch BracketSubmitted logs")
    }

    pub async fn get_tag_set_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(contract, TagSet::SIGNATURE_HASH, from_block, to_block)
            .await
            .wrap_err("failed to fetch TagSet logs")
    }

    // ── BracketGroups logs ───────────────────────────────────────────

    pub async fn get_group_created_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(contract, GroupCreated::SIGNATURE_HASH, from_block, to_block)
            .await
            .wrap_err("failed to fetch GroupCreated logs")
    }

    pub async fn get_member_joined_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(contract, MemberJoined::SIGNATURE_HASH, from_block, to_block)
            .await
            .wrap_err("failed to fetch MemberJoined logs")
    }

    pub async fn get_member_left_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(contract, MemberLeft::SIGNATURE_HASH, from_block, to_block)
            .await
            .wrap_err("failed to fetch MemberLeft logs")
    }

    // ── BracketMirror logs ───────────────────────────────────────────

    pub async fn get_mirror_created_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(
            contract,
            MirrorCreated::SIGNATURE_HASH,
            from_block,
            to_block,
        )
        .await
        .wrap_err("failed to fetch MirrorCreated logs")
    }

    pub async fn get_entry_added_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(contract, EntryAdded::SIGNATURE_HASH, from_block, to_block)
            .await
            .wrap_err("failed to fetch EntryAdded logs")
    }

    pub async fn get_entry_removed_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(contract, EntryRemoved::SIGNATURE_HASH, from_block, to_block)
            .await
            .wrap_err("failed to fetch EntryRemoved logs")
    }

    pub async fn get_bracket_updated_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        self.get_logs_for_event(
            contract,
            BracketUpdated::SIGNATURE_HASH,
            from_block,
            to_block,
        )
        .await
        .wrap_err("failed to fetch BracketUpdated logs")
    }

    // ── Contract calls ───────────────────────────────────────────────

    /// Call `getEntryCount()` on the MarchMadness contract.
    pub async fn get_entry_count(&self, contract: Address) -> Result<u32> {
        let calldata = getEntryCountCall {}.abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getEntryCount call failed")?;
        let count = getEntryCountCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getEntryCount result")?;
        Ok(count)
    }

    /// Call `getBracket(address)` on the MarchMadness contract.
    pub async fn get_bracket(&self, contract: Address, account: Address) -> Result<FixedBytes<8>> {
        let calldata = getBracketCall { account }.abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getBracket call failed")?;
        let bracket = getBracketCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getBracket result")?;

        if bracket == FixedBytes::ZERO {
            bail!("getBracket returned zero for {}", account);
        }
        Ok(bracket)
    }

    /// Call `getGroup(groupId)` on the BracketGroups contract and return the on-chain member count.
    pub async fn get_group_member_count(&self, contract: Address, group_id: u32) -> Result<u32> {
        let calldata = getGroupCall { groupId: group_id }.abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getGroup call failed")?;
        let decoded = getGroupCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getGroup result")?;
        Ok(decoded.entryCount)
    }

    /// Call `getGroup(groupId)` on the BracketGroups contract and return the entry fee as a
    /// decimal string (wei).
    pub async fn get_group_entry_fee(&self, contract: Address, group_id: u32) -> Result<String> {
        let calldata = getGroupCall { groupId: group_id }.abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getGroup call failed")?;
        let decoded = getGroupCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getGroup result")?;
        Ok(decoded.entryFee.to_string())
    }

    /// Call `nextMirrorId()` on the BracketMirror contract.
    pub async fn get_next_mirror_id(&self, contract: Address) -> Result<u64> {
        let calldata = nextMirrorIdCall {}.abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("nextMirrorId call failed")?;
        let next_id = nextMirrorIdCall::abi_decode_returns(&response)
            .wrap_err("failed to decode nextMirrorId result")?;
        let id: u64 = next_id.try_into().wrap_err("nextMirrorId exceeds u64")?;
        Ok(id)
    }

    /// Call `getMirror(mirrorId)` on the BracketMirror contract.
    pub async fn get_mirror(
        &self,
        contract: Address,
        mirror_id: alloy_primitives::U256,
    ) -> Result<Mirror> {
        let calldata = getMirrorCall {
            mirrorId: mirror_id,
        }
        .abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getMirror call failed")?;
        let mirror = getMirrorCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getMirror result")?;
        Ok(mirror)
    }

    /// Call `getEntries(mirrorId)` on the BracketMirror contract.
    pub async fn get_mirror_entries(
        &self,
        contract: Address,
        mirror_id: alloy_primitives::U256,
    ) -> Result<Vec<MirrorEntry>> {
        let calldata = getEntriesCall {
            mirrorId: mirror_id,
        }
        .abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getEntries call failed")?;
        let decoded = getEntriesCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getEntries result")?;
        Ok(decoded)
    }

    /// Call `getEntryBySlug(mirrorId, slug)` on the BracketMirror contract.
    pub async fn get_mirror_entry_bracket(
        &self,
        contract: Address,
        mirror_id: alloy_primitives::U256,
        slug: String,
    ) -> Result<FixedBytes<8>> {
        let calldata = getEntryBySlugCall {
            mirrorId: mirror_id,
            slug,
        }
        .abi_encode();
        let response = match self {
            Self::Reth(p) => {
                let tx = build_reth_call_tx(contract, calldata);
                p.call(tx).await
            }
            Self::Foundry(p) => {
                let tx = build_foundry_call_tx(contract, calldata);
                p.call(tx).await
            }
        }
        .wrap_err("getEntryBySlug call failed")?;
        let decoded = getEntryBySlugCall::abi_decode_returns(&response)
            .wrap_err("failed to decode getEntryBySlug result")?;
        Ok(decoded.bracket)
    }
}

/// Extract the address from a BracketSubmitted log.
pub fn parse_bracket_submitted(log: &Log) -> Result<Address> {
    let decoded = BracketSubmitted::decode_log(log.inner.as_ref())
        .wrap_err("failed to decode BracketSubmitted event")?;
    Ok(decoded.account)
}

/// Extract the address and tag from a TagSet log.
pub fn parse_tag_set(log: &Log) -> Result<(Address, String)> {
    let decoded =
        TagSet::decode_log(log.inner.as_ref()).wrap_err("failed to decode TagSet event")?;
    Ok((decoded.account, decoded.tag.clone()))
}

/// Parse a GroupCreated log.
pub fn parse_group_created(log: &Log) -> Result<(u32, String, String, Address, bool)> {
    let decoded = GroupCreated::decode_log(log.inner.as_ref())
        .wrap_err("failed to decode GroupCreated event")?;
    Ok((
        decoded.groupId,
        decoded.slug.clone(),
        decoded.displayName.clone(),
        decoded.creator,
        decoded.hasPassword,
    ))
}

/// Parse a MemberJoined log.
pub fn parse_member_joined(log: &Log) -> Result<(u32, Address)> {
    let decoded = MemberJoined::decode_log(log.inner.as_ref())
        .wrap_err("failed to decode MemberJoined event")?;
    Ok((decoded.groupId, decoded.addr))
}

/// Parse a MemberLeft log.
pub fn parse_member_left(log: &Log) -> Result<(u32, Address)> {
    let decoded =
        MemberLeft::decode_log(log.inner.as_ref()).wrap_err("failed to decode MemberLeft event")?;
    Ok((decoded.groupId, decoded.addr))
}

/// Parse a MirrorCreated log.
pub fn parse_mirror_created(log: &Log) -> Result<(u64, String, String, Address)> {
    let decoded = MirrorCreated::decode_log(log.inner.as_ref())
        .wrap_err("failed to decode MirrorCreated event")?;
    // mirrorId is U256, but we know it fits in u64
    let id: u64 = decoded
        .mirrorId
        .try_into()
        .wrap_err("mirrorId exceeds u64")?;
    Ok((
        id,
        decoded.slug.clone(),
        decoded.displayName.clone(),
        decoded.admin,
    ))
}

/// Parse an EntryAdded log (slug-based).
pub fn parse_entry_added(log: &Log) -> Result<(u64, String)> {
    let decoded =
        EntryAdded::decode_log(log.inner.as_ref()).wrap_err("failed to decode EntryAdded event")?;
    let id: u64 = decoded
        .mirrorId
        .try_into()
        .wrap_err("mirrorId exceeds u64")?;
    Ok((id, decoded.slug.clone()))
}

/// Parse an EntryRemoved log (slug-based).
pub fn parse_entry_removed(log: &Log) -> Result<(u64, String)> {
    let decoded = EntryRemoved::decode_log(log.inner.as_ref())
        .wrap_err("failed to decode EntryRemoved event")?;
    let id: u64 = decoded
        .mirrorId
        .try_into()
        .wrap_err("mirrorId exceeds u64")?;
    Ok((id, decoded.slug.clone()))
}

/// Parse a BracketUpdated log.
pub fn parse_bracket_updated(log: &Log) -> Result<(u64, String)> {
    let decoded = BracketUpdated::decode_log(log.inner.as_ref())
        .wrap_err("failed to decode BracketUpdated event")?;
    let id: u64 = decoded
        .mirrorId
        .try_into()
        .wrap_err("mirrorId exceeds u64")?;
    Ok((id, decoded.slug.clone()))
}

/// Build a SeismicReth transaction request for eth_call.
fn build_reth_call_tx(contract: Address, calldata: Vec<u8>) -> SeismicTransactionRequest {
    let input = Bytes::from(calldata);
    <SeismicTransactionRequest as TransactionBuilder<SeismicReth>>::with_input(
        <SeismicTransactionRequest as TransactionBuilder<SeismicReth>>::with_to(
            SeismicTransactionRequest::default(),
            contract,
        ),
        input,
    )
}

/// Build a SeismicFoundry transaction request for eth_call.
fn build_foundry_call_tx(
    contract: Address,
    calldata: Vec<u8>,
) -> seismic_alloy_network::foundry::tx_request::SeismicFoundryTransactionRequest {
    use seismic_alloy_network::foundry::tx_request::SeismicFoundryTransactionRequest;
    let input = Bytes::from(calldata);
    <SeismicFoundryTransactionRequest as TransactionBuilder<SeismicFoundry>>::with_input(
        <SeismicFoundryTransactionRequest as TransactionBuilder<SeismicFoundry>>::with_to(
            SeismicFoundryTransactionRequest::default(),
            contract,
        ),
        input,
    )
}
