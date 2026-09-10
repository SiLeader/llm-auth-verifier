mod download;
mod key;

use serde::Deserialize;
use std::collections::HashSet;

pub use key::JwtVerifier;

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    issuer: String,
    audience: String,
    subjects: HashSet<String>,
}
