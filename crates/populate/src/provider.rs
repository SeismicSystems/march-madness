//! Seismic provider helpers for the populate binary.
//!
//! `ReadProvider` — unsigned, for reading V1 source contracts.
//! `SignedProvider` — signed, for writing to V2 target contracts.

use alloy_signer_local::PrivateKeySigner;
use alloy_transport_http::reqwest;
use eyre::{Result, WrapErr};
use seismic_alloy_network::{SeismicReth, foundry::SeismicFoundry, wallet::SeismicWallet};
use seismic_alloy_provider::{
    SeismicProviderBuilder, SeismicSignedProvider, SeismicUnsignedProvider,
};

/// Unsigned provider for reading source contracts.
pub enum ReadProvider {
    Reth(SeismicUnsignedProvider<SeismicReth>),
    Foundry(SeismicUnsignedProvider<SeismicFoundry>),
}

impl ReadProvider {
    pub fn new_reth(rpc_url: &str) -> Result<Self> {
        let url: reqwest::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
        Ok(Self::Reth(SeismicProviderBuilder::new().connect_http(url)))
    }

    pub fn new_foundry(rpc_url: &str) -> Result<Self> {
        let url: reqwest::Url = rpc_url.parse().wrap_err("invalid RPC URL")?;
        Ok(Self::Foundry(
            SeismicProviderBuilder::new().foundry().connect_http(url),
        ))
    }
}

/// Signed provider for writing to target contracts.
pub enum SignedProvider {
    Reth(SeismicSignedProvider<SeismicReth>),
    Foundry(SeismicSignedProvider<SeismicFoundry>),
}

impl SignedProvider {
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
