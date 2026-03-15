use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root")
        .to_path_buf()
}

pub struct KalshiAuth {
    pub api_key: String,
    pub private_key: RsaPrivateKey,
}

impl KalshiAuth {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let root = workspace_root();

        let env_path = root.join(".env");
        let env_content = fs::read_to_string(&env_path)
            .map_err(|e| format!("Failed to read {}: {}", env_path.display(), e))?;
        let api_key = env_content
            .lines()
            .find_map(|line| line.strip_prefix("KALSHI_PUBLIC_KEY="))
            .ok_or("KALSHI_PUBLIC_KEY not found in .env")?
            .trim()
            .to_string();

        let pk_path = root.join("kalshi-rsa-pk");
        let pk_pem = fs::read_to_string(&pk_path)
            .map_err(|e| format!("Failed to read {}: {}", pk_path.display(), e))?;
        let private_key = RsaPrivateKey::from_pkcs1_pem(pk_pem.trim())
            .map_err(|e| format!("Failed to parse RSA key: {}", e))?;

        Ok(Self {
            api_key,
            private_key,
        })
    }

    /// Sign a request: returns (timestamp_ms_string, base64_signature).
    pub fn sign(
        &self,
        method: &str,
        path: &str,
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        let timestamp = Utc::now().timestamp_millis().to_string();
        let message = format!("{}{}{}", timestamp, method, path);
        let digest = Sha256::digest(message.as_bytes());
        let signature = self
            .private_key
            .sign(Pkcs1v15Sign::new::<Sha256>(), &digest)?;
        let sig_b64 = general_purpose::STANDARD.encode(&signature);
        Ok((timestamp, sig_b64))
    }

    /// Build HTTP headers for REST or WS authentication.
    pub fn auth_headers(
        &self,
        method: &str,
        path: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let (ts, sig) = self.sign(method, path)?;
        Ok(vec![
            ("KALSHI-ACCESS-KEY".to_string(), self.api_key.clone()),
            ("KALSHI-ACCESS-SIGNATURE".to_string(), sig),
            ("KALSHI-ACCESS-TIMESTAMP".to_string(), ts),
        ])
    }
}
