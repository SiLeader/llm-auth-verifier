use crate::jwt::JwtConfig;
use crate::jwt::download::JwkDownloader;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::Algorithm;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_CACHE_TTL: Duration = Duration::hours(12);

#[derive(Debug, Clone)]
pub(super) struct JwkIssuer {
    config: JwtConfig,
    cache_ttl: Duration,
    keys: Arc<RwLock<KeyCache>>,
}

#[derive(Debug, Clone)]
struct KeyCache {
    last_fetched: DateTime<Utc>,
    keys: HashMap<String, jsonwebtoken::DecodingKey>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
}

impl JwkIssuer {
    pub(super) fn new(config: JwtConfig, keys: HashMap<String, jsonwebtoken::DecodingKey>) -> Self {
        let now = Utc::now();
        Self {
            cache_ttl: config.cache_ttl.map(|d| d.0).unwrap_or(DEFAULT_CACHE_TTL),
            config,
            keys: Arc::new(RwLock::new(KeyCache {
                last_fetched: now,
                keys,
            })),
        }
    }

    pub(super) fn issuer(&self) -> &str {
        &self.config.issuer
    }

    pub(super) async fn verify(
        &self,
        alg: Algorithm,
        kid: &str,
        token: &str,
    ) -> anyhow::Result<bool> {
        let keys = self.keys.read().await;
        if let Some(decoding_key) = keys.keys.get(kid) {
            let validation = {
                let mut v = jsonwebtoken::Validation::new(alg);
                v.set_audience(&self.config.audiences);
                v.set_issuer(&[&self.config.issuer]);
                v.set_required_spec_claims(&["sub", "aud", "iss", "exp"]);
                v.validate_exp = true;
                v.validate_nbf = true;
                v.validate_aud = true;
                v
            };
            let claims =
                jsonwebtoken::decode::<Claims>(token, decoding_key, &validation).map_err(|e| {
                    anyhow::anyhow!(
                        "Token verification failed for issuer {}: {}",
                        self.config.issuer,
                        e
                    )
                })?;

            if !self.config.subjects.contains(&claims.claims.sub) {
                anyhow::bail!(
                    "Token claims do not match expected values for issuer {}",
                    self.config.issuer
                );
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(super) async fn refresh_keys(&self) -> anyhow::Result<()> {
        let mut keys = self.keys.write().await;
        if keys.last_fetched + self.cache_ttl > Utc::now() {
            return Ok(());
        }
        let downloader = JwkDownloader::new(&self.config.issuer);
        let ks = downloader.load().await?;
        keys.keys.clear();
        keys.keys.extend(ks);
        keys.last_fetched = Utc::now();
        Ok(())
    }
}
