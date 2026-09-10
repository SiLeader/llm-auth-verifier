use chrono::{DateTime, Utc};
use constant_time_eq::constant_time_eq;
use serde::Deserialize;
use sha2::Digest;

#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    pub raw: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
    pub expire_at: Option<DateTime<Utc>>,
    pub api: Option<ApiType>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiType {
    OpenAi,
    Anthropic,
}

impl Token {
    pub fn verify_config(&self) -> anyhow::Result<()> {
        if self.raw.is_none() && self.sha256.is_none() && self.sha512.is_none() {
            anyhow::bail!("At least one of 'raw', 'sha256', or 'sha512' is required.");
        }
        if let Some(raw) = &self.raw
            && raw.is_empty()
        {
            anyhow::bail!("'raw' token cannot be empty.");
        }
        if let Some(sha256) = &self.sha256
            && sha256.len() != 64
        {
            anyhow::bail!("'sha256' must be a 64-character hex string.");
        }
        if let Some(sha512) = &self.sha512
            && sha512.len() != 128
        {
            anyhow::bail!("'sha512' must be a 128-character hex string.");
        }
        Ok(())
    }

    pub fn verify(&self, api: ApiType, token: &str) -> bool {
        if let Some(at) = self.api
            && at != api
        {
            return false;
        }
        if let Some(exp) = &self.expire_at {
            let now = Utc::now();
            if exp < &now {
                return false;
            }
        }
        if let Some(raw) = &self.raw
            && !constant_time_eq(raw.as_bytes(), token.as_bytes())
        {
            return false;
        }
        if let Some(sha256) = &self.sha256 {
            let token_sha256 = hex::encode(sha2::Sha256::digest(token.as_bytes()));
            if !constant_time_eq(token_sha256.as_bytes(), sha256.as_bytes()) {
                return false;
            }
        }
        if let Some(sha512) = &self.sha512 {
            let token_sha512 = hex::encode(sha2::Sha512::digest(token.as_bytes()));
            if !constant_time_eq(token_sha512.as_bytes(), sha512.as_bytes()) {
                return false;
            }
        }
        true
    }
}
