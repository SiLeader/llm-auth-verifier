use crate::api::AiApi;
use crate::token::TokenConfig;
use chrono::{DateTime, Utc};
use constant_time_eq::constant_time_eq;
use sha2::Digest;
use std::collections::HashSet;
use tracing::info;

#[derive(Debug, Clone)]
pub struct TokenVerifier {
    tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
struct Token {
    name: String,
    allowed_apis: Option<HashSet<String>>,
    expire_at: Option<DateTime<Utc>>,
    content: TokenContent,
}

#[derive(Debug, Clone)]
enum TokenContent {
    Raw(String),
    Sha256(Vec<u8>),
    Sha512(Vec<u8>),
}

impl TokenVerifier {
    pub fn try_new(config: Vec<TokenConfig>) -> anyhow::Result<Self> {
        let tokens: Vec<Token> = config
            .into_iter()
            .enumerate()
            .map(|(index, tc)| Token::try_new(index, tc))
            .collect::<anyhow::Result<_>>()?;
        Ok(Self { tokens })
    }

    pub fn verify(&self, api: &AiApi, token: &str) -> bool {
        for tk in &self.tokens {
            if tk.verify(api, token) {
                return true;
            }
        }
        false
    }
}

impl Token {
    fn try_new(index: usize, value: TokenConfig) -> anyhow::Result<Self> {
        if value.raw.is_none() && value.sha256.is_none() && value.sha512.is_none() {
            anyhow::bail!("At least one of 'raw', 'sha256', or 'sha512' is required.");
        }
        let name = value.name.unwrap_or_else(|| format!("index:{index}"));
        if let Some(sha512) = value.sha512 {
            let digest = hex::decode(sha512)?;
            Ok(Self {
                name,
                allowed_apis: value.allowed_apis,
                expire_at: value.expire_at,
                content: TokenContent::Sha512(digest),
            })
        } else if let Some(sha256) = value.sha256 {
            let digest = hex::decode(sha256)?;
            Ok(Self {
                name,
                allowed_apis: value.allowed_apis,
                expire_at: value.expire_at,
                content: TokenContent::Sha256(digest),
            })
        } else if let Some(raw) = value.raw {
            Ok(Self {
                name,
                allowed_apis: value.allowed_apis,
                expire_at: None,
                content: TokenContent::Raw(raw),
            })
        } else {
            anyhow::bail!("At least one of 'raw', 'sha256', or 'sha512' is required.");
        }
    }

    fn verify(&self, api: &AiApi, token: &str) -> bool {
        if let Some(at) = &self.allowed_apis
            && at.contains(api.name())
        {
            return false;
        }
        if let Some(exp) = &self.expire_at {
            let now = Utc::now();
            if exp < &now {
                return false;
            }
        }
        let is_ok = match &self.content {
            TokenContent::Raw(content) => constant_time_eq(content.as_bytes(), token.as_bytes()),
            TokenContent::Sha256(content) => {
                let digest = sha2::Sha256::digest(token.as_bytes());
                constant_time_eq(content.as_slice(), digest.as_slice())
            }
            TokenContent::Sha512(content) => {
                let digest = sha2::Sha512::digest(token.as_bytes());
                constant_time_eq(content.as_slice(), digest.as_slice())
            }
        };
        info!(
            audit=true,
            name=%self.name,
            auth_type="token",
            allowed=%is_ok,
            api_name=%api.name()
        );
        is_ok
    }
}
