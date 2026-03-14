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

use crate::contract::{BracketSubmitted, TagSet, getBracketCall, getEntryCountCall};

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

    /// Fetch BracketSubmitted event logs in a block range.
    pub async fn get_bracket_submitted_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .address(contract)
            .event_signature(BracketSubmitted::SIGNATURE_HASH)
            .from_block(from_block)
            .to_block(to_block);

        let logs = match self {
            Self::Reth(p) => p.get_logs(&filter).await,
            Self::Foundry(p) => p.get_logs(&filter).await,
        }
        .wrap_err("failed to fetch BracketSubmitted logs")?;
        Ok(logs)
    }

    /// Fetch TagSet event logs in a block range.
    pub async fn get_tag_set_logs(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .address(contract)
            .event_signature(TagSet::SIGNATURE_HASH)
            .from_block(from_block)
            .to_block(to_block);

        let logs = match self {
            Self::Reth(p) => p.get_logs(&filter).await,
            Self::Foundry(p) => p.get_logs(&filter).await,
        }
        .wrap_err("failed to fetch TagSet logs")?;
        Ok(logs)
    }

    /// Call `getEntryCount()` on the contract.
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

    /// Call `getBracket(address)` on the contract.
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
