mod extract;

use crate::api::{AiApi, ApiDetector};
use crate::config::Config;
use crate::jwt::JwtVerifier;
use crate::token::TokenVerifier;
use axum::http::HeaderMap;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Verifier {
    api_detector: ApiDetector,
    token_verifier: TokenVerifier,
    jwt_verifier: JwtVerifier,
}

impl Verifier {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let api_detector = ApiDetector::try_new(config.api.unwrap_or_default())?;
        let token_verifier = TokenVerifier::try_new(config.tokens)?;
        let jwt_verifier = JwtVerifier::load(config.oidc).await?;
        Ok(Self {
            api_detector,
            token_verifier,
            jwt_verifier,
        })
    }

    pub async fn verify_token(&self, headers: &HeaderMap) -> bool {
        let Some((api, token)) = self.extract_api(headers) else {
            warn!("api does not match token headers");
            return false;
        };
        self.verify_token_impl(api, token).await
    }

    async fn verify_token_impl(&self, api: &AiApi, token: &str) -> bool {
        if self.token_verifier.verify(api, token) {
            return true;
        }
        if let Err(e) = self.jwt_verifier.verify(api, token).await {
            warn!("JWT verification failed: {}", e);
            false
        } else {
            true
        }
    }
}
