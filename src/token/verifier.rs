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
    api: Option<crate::config::ApiType>,
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
                api: value.api,
                expire_at: value.expire_at,
                content: TokenContent::Sha512(digest),
            })
        } else if let Some(sha256) = value.sha256 {
            let digest = hex::decode(sha256)?;
            Ok(Self {
                name,
                allowed_apis: value.allowed_apis,
                api: value.api,
                expire_at: value.expire_at,
                content: TokenContent::Sha256(digest),
            })
        } else if let Some(raw) = value.raw {
            Ok(Self {
                name,
                allowed_apis: value.allowed_apis,
                api: value.api,
                expire_at: value.expire_at,
                content: TokenContent::Raw(raw),
            })
        } else {
            anyhow::bail!("At least one of 'raw', 'sha256', or 'sha512' is required.");
        }
    }

    fn verify(&self, api: &AiApi, token: &str) -> bool {
        if let Some(at) = &self.allowed_apis
            && !at.contains(api.name())
        {
            return false;
        }
        if let Some(expected) = self.api
            && api.api_type() != Some(expected)
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

#[cfg(test)]
mod tests {
    use super::TokenVerifier;
    use crate::api::{AiApi, ApiPath};
    use crate::config::ApiType;
    use crate::token::TokenConfig;
    use axum::http::Method;
    use chrono::{Duration, Utc};
    use sha2::Digest;
    use std::collections::HashSet;

    fn config(raw: &str) -> TokenConfig {
        TokenConfig {
            name: Some("test".to_owned()),
            allowed_apis: None,
            raw: Some(raw.to_owned()),
            sha256: None,
            sha512: None,
            expire_at: None,
            api: None,
        }
    }

    fn openai_api(name: &str) -> AiApi {
        AiApi::openai(name, Method::POST, ApiPath::exact("/openai"))
    }

    fn anthropic_api(name: &str) -> AiApi {
        AiApi::anthropic(name, Method::POST, ApiPath::exact("/anthropic"))
    }

    #[test]
    fn verifies_raw_and_hashed_tokens() {
        let raw = config("raw-secret");
        let mut sha256 = config("");
        sha256.raw = None;
        sha256.sha256 = Some(hex::encode(sha2::Sha256::digest(b"hashed-secret")));
        let mut sha512 = config("");
        sha512.raw = None;
        sha512.sha512 = Some(hex::encode(sha2::Sha512::digest(b"hashed-secret")));
        let verifier = TokenVerifier::try_new(vec![raw, sha256, sha512]).unwrap();
        let api = openai_api("openai/chat-completions");

        assert!(verifier.verify(&api, "raw-secret"));
        assert!(verifier.verify(&api, "hashed-secret"));
        assert!(!verifier.verify(&api, "wrong-secret"));
    }

    #[test]
    fn allowed_apis_restricts_token_to_listed_names() {
        let mut token = config("secret");
        token.allowed_apis = Some(HashSet::from(["openai/chat-completions".to_owned()]));
        let verifier = TokenVerifier::try_new(vec![token]).unwrap();

        assert!(verifier.verify(&openai_api("openai/chat-completions"), "secret"));
        assert!(!verifier.verify(&openai_api("openai/responses"), "secret"));
    }

    #[test]
    fn api_type_restricts_token_to_matching_api_family() {
        let mut token = config("secret");
        token.api = Some(ApiType::Anthropic);
        let verifier = TokenVerifier::try_new(vec![token]).unwrap();

        assert!(verifier.verify(&anthropic_api("anthropic/messages"), "secret"));
        assert!(!verifier.verify(&openai_api("openai/chat-completions"), "secret"));
    }

    #[test]
    fn expired_raw_token_is_rejected() {
        let mut token = config("secret");
        token.expire_at = Some(Utc::now() - Duration::seconds(1));
        let verifier = TokenVerifier::try_new(vec![token]).unwrap();

        assert!(!verifier.verify(&openai_api("openai/chat-completions"), "secret"));
    }

    #[test]
    fn token_without_content_is_rejected() {
        let mut token = config("");
        token.raw = None;

        assert!(TokenVerifier::try_new(vec![token]).is_err());
    }
}
