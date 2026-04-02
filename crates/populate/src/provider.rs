//! Signed Seismic provider for the populate binary.
//!
//! Wraps either SeismicReth (production/testnet) or SeismicFoundry (sanvil)
//! as a signed provider capable of sending transactions.

use alloy_signer_local::PrivateKeySigner;
use alloy_transport_http::reqwest;
use eyre::{Result, WrapErr};
use seismic_alloy_network::{SeismicReth, foundry::SeismicFoundry, wallet::SeismicWallet};
use seismic_alloy_provider::{SeismicProviderBuilder, SeismicSignedProvider};

/// Network-agnostic signed provider.
pub enum SignedProvider {
    Reth(SeismicSignedProvider<SeismicReth>),
    Foundry(SeismicSignedProvider<SeismicFoundry>),
}

impl SignedProvider {
    /// Create a signed provider for SeismicReth (production / testnet).
    pub async fn new_reth(rpc_url: &str, private_key: &str) -> Result<Self> {
        let url: reqwest::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
        let signer: PrivateKeySigner = private_key.parse().wrap_err("invalid private key")?;
        let wallet = SeismicWallet::<SeismicReth>::from(signer);
        let provider = SeismicProviderBuilder::new()
            .wallet(wallet)
            .connect_http(url)
            .await
            .map_err(|e| eyre::eyre!("failed to connect signed reth provider: {e}"))?;
        Ok(Self::Reth(provider))
    }

    /// Create a signed provider for SeismicFoundry / sanvil (local development).
    pub async fn new_foundry(rpc_url: &str, private_key: &str) -> Result<Self> {
        let url: reqwest::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
        let signer: PrivateKeySigner = private_key.parse().wrap_err("invalid private key")?;
        let wallet = SeismicWallet::<SeismicFoundry>::from(signer);
        let provider = SeismicProviderBuilder::new()
            .foundry()
            .wallet(wallet)
            .connect_http(url)
            .await
            .map_err(|e| eyre::eyre!("failed to connect signed foundry provider: {e}"))?;
        Ok(Self::Foundry(provider))
    }
}
