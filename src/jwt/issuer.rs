use crate::api::AiApi;
use crate::jwt::JwtConfig;
use crate::jwt::download::JwkDownloader;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::Algorithm;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

const DEFAULT_CACHE_TTL: Duration = Duration::hours(12);
const MIN_REFRESH_INTERVAL: Duration = Duration::minutes(5);

#[derive(Debug, Clone)]
pub(super) struct JwkIssuer {
    config: JwtConfig,
    cache_ttl: Duration,
    keys: Arc<RwLock<KeyCache>>,
    refresh_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone)]
struct KeyCache {
    last_fetched: DateTime<Utc>,
    last_refresh_attempt: Option<DateTime<Utc>>,
    keys: HashMap<String, jsonwebtoken::DecodingKey>,
}

impl JwkIssuer {
    pub(super) fn new(config: JwtConfig, keys: HashMap<String, jsonwebtoken::DecodingKey>) -> Self {
        let now = Utc::now();
        Self {
            cache_ttl: config.cache_ttl.map(|d| d.0).unwrap_or(DEFAULT_CACHE_TTL),
            config,
            keys: Arc::new(RwLock::new(KeyCache {
                last_fetched: now,
                last_refresh_attempt: None,
                keys,
            })),
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    pub(super) fn name(&self) -> &str {
        &self.config.issuer
    }

    pub(super) async fn verify(
        &self,
        api: &AiApi,
        alg: Algorithm,
        kid: &str,
        token: &str,
    ) -> anyhow::Result<bool> {
        if !self.config.algorithms.contains(&alg) {
            anyhow::bail!(
                "Algorithm {:?} is not allowed for issuer {}",
                alg,
                self.config.issuer
            );
        }

        if let Err(error) = self.refresh_if_stale().await {
            warn!(
                "Failed to refresh stale keys for issuer {}: {}",
                self.config.issuer, error
            );
        }

        if self.verify_cached(api, alg, kid, token).await? {
            return Ok(true);
        }

        self.refresh_for_unknown_kid(kid).await?;
        self.verify_cached(api, alg, kid, token).await
    }

    async fn verify_cached(
        &self,
        api: &AiApi,
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
                v.set_required_spec_claims(&["aud", "iss", "exp"]);
                v.validate_exp = true;
                v.validate_nbf = true;
                v.validate_aud = true;
                v
            };
            let claims =
                jsonwebtoken::decode::<serde_json::Value>(token, decoding_key, &validation)
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Token verification failed for issuer {}: {}",
                            self.config.issuer,
                            e
                        )
                    })?;

            if !self
                .config
                .access_rules
                .iter()
                .any(|rule| rule.allows(api.name(), &claims.claims))
            {
                anyhow::bail!(
                    "Token claims are not allowed to access API {} for issuer {}",
                    api.name(),
                    self.config.issuer
                );
            }

            let subject = claims
                .claims
                .get("sub")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<no-subject>");

            info!(
                audit=true,
                issuer=%self.name(),
                name=%subject,
                auth_type="oidc",
                allowed=true,
                api_name=%api.name()
            );

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn refresh_if_stale(&self) -> anyhow::Result<()> {
        let keys = self.keys.read().await;
        if keys.last_fetched + self.cache_ttl > Utc::now() {
            return Ok(());
        }
        drop(keys);

        self.refresh_keys(None).await
    }

    async fn refresh_for_unknown_kid(&self, kid: &str) -> anyhow::Result<()> {
        self.refresh_keys(Some(kid)).await
    }

    async fn refresh_keys(&self, missing_kid: Option<&str>) -> anyhow::Result<()> {
        if !self.refresh_needed(missing_kid, Utc::now()).await {
            return Ok(());
        }

        let refresh_guard = Arc::clone(&self.refresh_lock).lock_owned().await;
        let now = Utc::now();

        // Another request may have refreshed the cache while this request was
        // waiting for the single-flight lock.
        if !self.refresh_needed(missing_kid, now).await {
            return Ok(());
        }

        // Run the refresh on a detached task. The triggering request is dropped
        // when its client disconnects, and cancelling the fetch there would
        // start the cooldown without updating the keys. The task owns the
        // single-flight lock so waiting requests resume only after it finishes.
        let issuer = self.config.issuer.clone();
        let keys = Arc::clone(&self.keys);
        tokio::spawn(async move {
            let _refresh_guard = refresh_guard;
            keys.write().await.last_refresh_attempt = Some(now);

            // Do not hold the cache write lock during network I/O. Existing keys
            // remain usable while a refresh is in progress.
            let downloader = JwkDownloader::new(&issuer)?;
            let new_keys = downloader.load().await?;

            let mut keys = keys.write().await;
            keys.keys = new_keys;
            keys.last_fetched = Utc::now();
            anyhow::Ok(())
        })
        .await?
    }

    async fn refresh_needed(&self, missing_kid: Option<&str>, now: DateTime<Utc>) -> bool {
        let keys = self.keys.read().await;

        if missing_kid.is_some_and(|kid| keys.keys.contains_key(kid)) {
            return false;
        }

        if missing_kid.is_none() && keys.last_fetched + self.cache_ttl > now {
            return false;
        }

        // Count failed attempts too, otherwise an unavailable provider can be
        // hammered once per incoming request.
        !keys
            .last_refresh_attempt
            .is_some_and(|attempt| attempt + MIN_REFRESH_INTERVAL > now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::AccessRule;
    use axum::Json;
    use axum::Router;
    use axum::http::StatusCode;
    use axum::routing::get;
    use jsonwebtoken::{EncodingKey, Header};
    use serde_json::{Value, json};
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;
    use tokio::task::JoinHandle;

    /// Holds the JWKS response until the test releases it.
    #[derive(Clone, Default)]
    struct JwksGate {
        requested: Arc<Notify>,
        release: Arc<Notify>,
    }

    async fn start_jwks_server(
        jwks_status: StatusCode,
        jwks: Value,
        gate: Option<JwksGate>,
    ) -> (String, Arc<AtomicUsize>, JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let discovery_issuer = issuer.clone();
        let jwks_uri = format!("{issuer}/jwks");
        let request_count = Arc::new(AtomicUsize::new(0));
        let jwks_request_count = Arc::clone(&request_count);

        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || {
                    let issuer = discovery_issuer.clone();
                    let jwks_uri = jwks_uri.clone();
                    async move { Json(json!({ "issuer": issuer, "jwks_uri": jwks_uri })) }
                }),
            )
            .route(
                "/jwks",
                get(move || {
                    let jwks = jwks.clone();
                    let request_count = Arc::clone(&jwks_request_count);
                    let gate = gate.clone();
                    async move {
                        request_count.fetch_add(1, Ordering::SeqCst);
                        if let Some(gate) = gate {
                            gate.requested.notify_one();
                            gate.release.notified().await;
                        }
                        (jwks_status, Json(jwks))
                    }
                }),
            );

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (issuer, request_count, server)
    }

    fn issuer_with_keys(
        issuer: String,
        keys: HashMap<String, jsonwebtoken::DecodingKey>,
    ) -> JwkIssuer {
        JwkIssuer::new(
            JwtConfig {
                issuer,
                audiences: vec!["audience".to_owned()],
                algorithms: HashSet::from([Algorithm::RS256]),
                access_rules: vec![AccessRule {
                    claims: HashMap::from([("sub".to_owned(), json!("subject"))]),
                    allowed_apis: HashSet::from(["openai/chat-completions".to_owned()]),
                }],
                cache_ttl: None,
            },
            keys,
        )
    }

    fn signed_token(algorithm: Algorithm, claims: Value) -> String {
        let mut header = Header::new(algorithm);
        header.kid = Some("test-key".to_owned());
        jsonwebtoken::encode(&header, &claims, &EncodingKey::from_secret(b"secret")).unwrap()
    }

    #[tokio::test]
    async fn claim_rules_limit_each_identity_to_its_apis() {
        let mut issuer = issuer_with_keys(
            "https://auth.example.com".to_owned(),
            HashMap::from([(
                "test-key".to_owned(),
                jsonwebtoken::DecodingKey::from_secret(b"secret"),
            )]),
        );
        issuer.config.algorithms = HashSet::from([Algorithm::HS256]);
        issuer.config.access_rules = vec![
            AccessRule {
                claims: HashMap::from([("sub".to_owned(), json!("alice"))]),
                allowed_apis: HashSet::from(["openai/chat-completions".to_owned()]),
            },
            AccessRule {
                claims: HashMap::from([("department".to_owned(), json!("research"))]),
                allowed_apis: HashSet::from(["openai/models".to_owned()]),
            },
        ];
        let claims = json!({
            "sub": "alice",
            "department": "sales",
            "aud": "audience",
            "iss": "https://auth.example.com",
            "exp": Utc::now().timestamp() + 3600
        });
        let token = signed_token(Algorithm::HS256, claims);
        let chat = AiApi::openai(
            "openai/chat-completions",
            axum::http::Method::POST,
            crate::api::ApiPath::exact("/v1/chat/completions"),
        );
        let models = AiApi::openai(
            "openai/models",
            axum::http::Method::GET,
            crate::api::ApiPath::exact("/v1/models"),
        );

        assert!(
            issuer
                .verify(&chat, Algorithm::HS256, "test-key", &token)
                .await
                .unwrap()
        );
        assert!(
            issuer
                .verify(&models, Algorithm::HS256, "test-key", &token)
                .await
                .is_err()
        );

        let claims_without_subject = json!({
            "department": "research",
            "aud": "audience",
            "iss": "https://auth.example.com",
            "exp": Utc::now().timestamp() + 3600
        });
        let token_without_subject = signed_token(Algorithm::HS256, claims_without_subject);
        assert!(
            issuer
                .verify(
                    &models,
                    Algorithm::HS256,
                    "test-key",
                    &token_without_subject
                )
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn rejects_signature_algorithm_outside_allowlist() {
        let issuer = issuer_with_keys("https://auth.example.com".to_owned(), HashMap::new());
        let api = AiApi::openai(
            "openai/chat-completions",
            axum::http::Method::POST,
            crate::api::ApiPath::exact("/v1/chat/completions"),
        );

        assert!(
            issuer
                .verify(&api, Algorithm::HS256, "test-key", "token")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn concurrent_unknown_kids_trigger_only_one_refresh() {
        let (issuer, request_count, server) = start_jwks_server(
            StatusCode::OK,
            json!({
                "keys": [{
                    "kty": "RSA",
                    "kid": "rotated-key",
                    "n": "AQAB",
                    "e": "AQAB"
                }]
            }),
            None,
        )
        .await;
        let issuer = issuer_with_keys(issuer, HashMap::new());

        let mut refreshes = Vec::new();
        for _ in 0..8 {
            let issuer = issuer.clone();
            refreshes.push(tokio::spawn(async move {
                issuer.refresh_for_unknown_kid("rotated-key").await.unwrap();
            }));
        }
        for refresh in refreshes {
            refresh.await.unwrap();
        }

        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        assert!(issuer.keys.read().await.keys.contains_key("rotated-key"));

        // A different random kid inside the cooldown must not cause another
        // outbound request.
        issuer
            .refresh_for_unknown_kid("attacker-controlled-kid")
            .await
            .unwrap();
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn failed_refresh_keeps_existing_keys_and_starts_cooldown() {
        let (issuer, request_count, server) = start_jwks_server(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "keys": [] }),
            None,
        )
        .await;
        let mut original_keys = HashMap::new();
        original_keys.insert(
            "existing-key".to_owned(),
            jsonwebtoken::DecodingKey::from_rsa_components("AQAB", "AQAB").unwrap(),
        );
        let issuer = issuer_with_keys(issuer, original_keys);

        assert!(issuer.refresh_for_unknown_kid("unknown-key").await.is_err());
        assert!(issuer.keys.read().await.keys.contains_key("existing-key"));

        issuer
            .refresh_for_unknown_kid("another-unknown-key")
            .await
            .unwrap();
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn cancelled_request_still_completes_refresh() {
        let gate = JwksGate::default();
        let (issuer, request_count, server) = start_jwks_server(
            StatusCode::OK,
            json!({
                "keys": [{
                    "kty": "RSA",
                    "kid": "rotated-key",
                    "n": "AQAB",
                    "e": "AQAB"
                }]
            }),
            Some(gate.clone()),
        )
        .await;
        let issuer = issuer_with_keys(issuer, HashMap::new());

        // Drop the triggering request while the JWKS response is pending, as
        // hyper does when the client disconnects.
        let request = tokio::spawn({
            let issuer = issuer.clone();
            async move { issuer.refresh_for_unknown_kid("rotated-key").await }
        });
        gate.requested.notified().await;
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        gate.release.notify_one();

        // The single-flight lock is held until the refresh has finished.
        drop(issuer.refresh_lock.lock().await);
        assert!(issuer.keys.read().await.keys.contains_key("rotated-key"));
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        server.abort();
    }
}
