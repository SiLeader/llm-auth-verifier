use crate::config::{ApiType, Config};
use crate::jwt::JwtVerifier;
use crate::token::TokenVerifier;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Verifier {
    token_verifier: TokenVerifier,
    jwt_verifier: JwtVerifier,
}

impl Verifier {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let token_verifier = TokenVerifier::try_new(config.tokens)?;
        let jwt_verifier = JwtVerifier::load(config.oidc).await?;
        Ok(Self {
            token_verifier,
            jwt_verifier,
        })
    }

    pub async fn verify_token(&self, api_type: ApiType, token: &str) -> bool {
        if self.token_verifier.verify(api_type, token) {
            return true;
        }
        if let Err(e) = self.jwt_verifier.verify(token).await {
            warn!("JWT verification failed: {}", e);
            false
        } else {
            true
        }
    }
}
