mod download;
mod duration_deserializer;
mod issuer;
mod key;

use serde::Deserialize;
use std::collections::HashSet;

use crate::jwt::duration_deserializer::Dur;
pub use key::JwtVerifier;

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    issuer: String,
    audiences: Vec<String>,
    subjects: HashSet<String>,
    cache_ttl: Option<Dur>, // Time-to-live for cached keys in seconds
}
