//! JSON-RPC client helpers for Ethereum calls.

use alloy_primitives::{Address, FixedBytes};
use eyre::{Result, WrapErr, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A minimal JSON-RPC client over HTTP.
#[derive(Clone)]
pub struct RpcClient {
    url: String,
    client: reqwest::Client,
    next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: Option<Value>,
    error: Option<Value>,
}

/// An Ethereum log entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: String,
    #[serde(default)]
    pub transaction_hash: String,
}

impl RpcClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: reqwest::Client::new(),
            next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let req = RpcRequest {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
            id,
        };
        let resp: RpcResponse = self
            .client
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .wrap_err("RPC request failed")?
            .json()
            .await
            .wrap_err("Failed to parse RPC response")?;

        if let Some(err) = resp.error {
            bail!("RPC error: {}", err);
        }
        resp.result.ok_or_else(|| eyre::eyre!("RPC returned null"))
    }

    /// Fetch logs matching the given filter.
    pub async fn get_logs(
        &self,
        address: &str,
        topics: &[String],
        from_block: u64,
        to_block: &str,
    ) -> Result<Vec<Log>> {
        let topics_val: Vec<Value> = topics
            .iter()
            .map(|t| {
                if t.is_empty() {
                    Value::Null
                } else {
                    Value::String(t.clone())
                }
            })
            .collect();

        let filter = serde_json::json!({
            "address": address,
            "topics": topics_val,
            "fromBlock": format!("0x{:x}", from_block),
            "toBlock": to_block,
        });

        let result = self
            .call("eth_getLogs", serde_json::json!([filter]))
            .await?;
        let logs: Vec<Log> = serde_json::from_value(result).wrap_err("Failed to parse logs")?;
        Ok(logs)
    }

    /// Get the latest block number.
    pub async fn block_number(&self) -> Result<u64> {
        let result = self.call("eth_blockNumber", serde_json::json!([])).await?;
        let hex = result
            .as_str()
            .ok_or_else(|| eyre::eyre!("bad block number"))?;
        parse_hex_u64(hex)
    }

    /// Get block timestamp by block number.
    pub async fn get_block_timestamp(&self, block_num: u64) -> Result<u64> {
        let result = self
            .call(
                "eth_getBlockByNumber",
                serde_json::json!([format!("0x{:x}", block_num), false]),
            )
            .await?;
        let ts_hex = result
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| eyre::eyre!("no timestamp in block"))?;
        parse_hex_u64(ts_hex)
    }

    /// Make an eth_call.
    pub async fn eth_call(&self, to: &str, data: &str) -> Result<String> {
        let result = self
            .call(
                "eth_call",
                serde_json::json!([{"to": to, "data": data}, "latest"]),
            )
            .await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| eyre::eyre!("bad eth_call result"))
    }

    /// Call `getEntryCount()` on the contract — returns uint32.
    pub async fn get_entry_count(&self, contract: &str) -> Result<u32> {
        // selector = keccak256("getEntryCount()")[..4]
        let selector = &function_selector("getEntryCount()");
        let data = format!("0x{}", hex::encode(selector));
        let result = self.eth_call(contract, &data).await?;
        // Result is ABI-encoded uint32 (32 bytes, right-padded to 256 bits)
        let bytes = decode_hex_data(&result)?;
        if bytes.len() < 32 {
            bail!("getEntryCount returned too few bytes");
        }
        // uint32 is in the last 4 bytes of the 32-byte word
        let val = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        Ok(val)
    }

    /// Call `getBracket(address)` on the contract — returns bytes8.
    pub async fn get_bracket(&self, contract: &str, account: &Address) -> Result<FixedBytes<8>> {
        let selector = function_selector("getBracket(address)");
        // ABI-encode: selector + address padded to 32 bytes
        let mut calldata = Vec::with_capacity(4 + 32);
        calldata.extend_from_slice(&selector);
        calldata.extend_from_slice(&[0u8; 12]); // left-pad address to 32 bytes
        calldata.extend_from_slice(account.as_slice());
        let data = format!("0x{}", hex::encode(&calldata));
        let result = self.eth_call(contract, &data).await?;
        let bytes = decode_hex_data(&result)?;
        if bytes.len() < 32 {
            bail!("getBracket returned too few bytes");
        }
        // bytes8 is left-aligned in the 32-byte word
        let mut out = [0u8; 8];
        out.copy_from_slice(&bytes[0..8]);
        Ok(FixedBytes(out))
    }
}

/// Compute the 4-byte function selector from a canonical signature.
pub fn function_selector(sig: &str) -> [u8; 4] {
    use alloy_primitives::keccak256;
    let hash = keccak256(sig.as_bytes());
    let mut sel = [0u8; 4];
    sel.copy_from_slice(&hash[..4]);
    sel
}

/// Compute keccak256 of an event signature, returning the topic0 hex string.
pub fn event_topic(sig: &str) -> String {
    use alloy_primitives::keccak256;
    let hash = keccak256(sig.as_bytes());
    format!("0x{}", hex::encode(hash.as_slice()))
}

/// Parse a hex string (with 0x prefix) to u64.
pub fn parse_hex_u64(s: &str) -> Result<u64> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).wrap_err_with(|| format!("bad hex u64: {s}"))
}

/// Decode hex data string (with 0x prefix) to bytes.
pub fn decode_hex_data(s: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).wrap_err("bad hex data")
}

/// Extract an address from a 32-byte hex topic (0x-prefixed, address in last 20 bytes).
pub fn address_from_topic(topic: &str) -> Result<Address> {
    let bytes = decode_hex_data(topic)?;
    if bytes.len() < 20 {
        bail!("topic too short for address");
    }
    // Address is in the last 20 bytes of the 32-byte topic
    let start = bytes.len() - 20;
    let addr = Address::from_slice(&bytes[start..]);
    Ok(addr)
}

/// Decode an ABI-encoded string from log data.
/// Layout: offset (32 bytes) + length (32 bytes) + data (padded to 32-byte boundary).
pub fn decode_abi_string(data: &[u8]) -> Result<String> {
    if data.len() < 64 {
        bail!("ABI string data too short");
    }
    // First 32 bytes: offset to string data (should be 0x20 = 32)
    // Next 32 bytes at the offset: length
    let offset = u64::from_be_bytes([
        data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
    ]) as usize;
    if offset + 32 > data.len() {
        bail!("ABI string offset out of bounds");
    }
    let len_start = offset;
    let len = u64::from_be_bytes([
        data[len_start + 24],
        data[len_start + 25],
        data[len_start + 26],
        data[len_start + 27],
        data[len_start + 28],
        data[len_start + 29],
        data[len_start + 30],
        data[len_start + 31],
    ]) as usize;
    let str_start = offset + 32;
    if str_start + len > data.len() {
        bail!("ABI string data out of bounds");
    }
    String::from_utf8(data[str_start..str_start + len].to_vec()).wrap_err("invalid UTF-8 in tag")
}
