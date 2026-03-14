//! Seismic-alloy provider helpers for the indexer.

use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, Bytes, FixedBytes};
use alloy_provider::Provider;
use alloy_rpc_types_eth::{BlockNumberOrTag, Filter, Log};
use alloy_sol_types::{SolCall, SolEvent};
use eyre::{Result, WrapErr, bail};
use seismic_alloy_network::SeismicReth;
use seismic_alloy_provider::{SeismicProviderBuilder, SeismicUnsignedProvider};

use crate::contract::{BracketSubmitted, TagSet, getBracketCall, getEntryCountCall};

/// The concrete provider type used throughout the indexer.
pub type IndexerProvider = SeismicUnsignedProvider<SeismicReth>;

/// Create an HTTP provider for the given RPC URL.
pub fn create_provider(rpc_url: &str) -> Result<IndexerProvider> {
    let url: url::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
    // reqwest::Url is a re-export of url::Url, so this parse is safe
    let reqwest_url: reqwest::Url =
        reqwest::Url::parse(url.as_str()).wrap_err("invalid RPC URL")?;
    Ok(SeismicProviderBuilder::new().connect_http(reqwest_url))
}

/// Get the latest block number.
pub async fn block_number(provider: &IndexerProvider) -> Result<u64> {
    let num = provider
        .get_block_number()
        .await
        .wrap_err("failed to get block number")?;
    Ok(num)
}

/// Get block timestamp by block number.
pub async fn get_block_timestamp(provider: &IndexerProvider, block_num: u64) -> Result<u64> {
    let block = provider
        .get_block_by_number(BlockNumberOrTag::Number(block_num))
        .await
        .wrap_err("failed to get block")?
        .ok_or_else(|| eyre::eyre!("block {} not found", block_num))?;
    Ok(block.header.timestamp)
}

/// Fetch BracketSubmitted event logs in a block range.
pub async fn get_bracket_submitted_logs(
    provider: &IndexerProvider,
    contract: Address,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<Log>> {
    let filter = Filter::new()
        .address(contract)
        .event_signature(BracketSubmitted::SIGNATURE_HASH)
        .from_block(from_block)
        .to_block(to_block);

    let logs = provider
        .get_logs(&filter)
        .await
        .wrap_err("failed to fetch BracketSubmitted logs")?;
    Ok(logs)
}

/// Fetch TagSet event logs in a block range.
pub async fn get_tag_set_logs(
    provider: &IndexerProvider,
    contract: Address,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<Log>> {
    let filter = Filter::new()
        .address(contract)
        .event_signature(TagSet::SIGNATURE_HASH)
        .from_block(from_block)
        .to_block(to_block);

    let logs = provider
        .get_logs(&filter)
        .await
        .wrap_err("failed to fetch TagSet logs")?;
    Ok(logs)
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

/// Build a transaction request for an eth_call.
fn build_call_tx(
    contract: Address,
    calldata: Vec<u8>,
) -> seismic_alloy_rpc_types::SeismicTransactionRequest {
    use seismic_alloy_rpc_types::SeismicTransactionRequest;
    let input = Bytes::from(calldata);
    <SeismicTransactionRequest as TransactionBuilder<SeismicReth>>::with_input(
        <SeismicTransactionRequest as TransactionBuilder<SeismicReth>>::with_to(
            SeismicTransactionRequest::default(),
            contract,
        ),
        input,
    )
}

/// Call `getEntryCount()` on the contract.
pub async fn get_entry_count(provider: &IndexerProvider, contract: Address) -> Result<u32> {
    let calldata = getEntryCountCall {}.abi_encode();
    let tx = build_call_tx(contract, calldata);

    let response = provider
        .call(tx)
        .await
        .wrap_err("getEntryCount call failed")?;
    let count = getEntryCountCall::abi_decode_returns(&response)
        .wrap_err("failed to decode getEntryCount result")?;
    Ok(count)
}

/// Call `getBracket(address)` on the contract.
pub async fn get_bracket(
    provider: &IndexerProvider,
    contract: Address,
    account: Address,
) -> Result<FixedBytes<8>> {
    let calldata = getBracketCall { account }.abi_encode();
    let tx = build_call_tx(contract, calldata);

    let response = provider.call(tx).await.wrap_err("getBracket call failed")?;
    let bracket = getBracketCall::abi_decode_returns(&response)
        .wrap_err("failed to decode getBracket result")?;

    if bracket == FixedBytes::ZERO {
        bail!("getBracket returned zero for {}", account);
    }
    Ok(bracket)
}
