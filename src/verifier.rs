use crate::config::{ApiType, Config, Token};
use crate::jwt::JwtVerifier;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Verifier {
    tokens: Vec<Token>,
    jwt_verifier: JwtVerifier,
}

impl Verifier {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        config.verify_config()?;
        let jwt_verifier = JwtVerifier::load(&config.oidc).await?;
        Ok(Self {
            tokens: config.tokens,
            jwt_verifier,
        })
    }

    pub fn verify_token(&self, api_type: ApiType, token: &str) -> bool {
        for t in &self.tokens {
            if t.verify(api_type, token) {
                return true;
            }
        }
        if let Err(e) = self.jwt_verifier.verify(token) {
            warn!("JWT verification failed: {}", e);
            false
        } else {
            true
        }
    }
}
