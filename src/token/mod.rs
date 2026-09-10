mod verifier;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::config::ApiType;
pub use verifier::TokenVerifier;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenConfig {
    pub raw: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
    pub expire_at: Option<DateTime<Utc>>,
    pub api: Option<ApiType>,
}
