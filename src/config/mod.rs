pub use crate::config::token::*;
use crate::jwt::JwtConfig;
use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

mod token;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tokens: Vec<Token>,
    #[serde(default)]
    pub oidc: Vec<JwtConfig>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;
        toml::from_str(&content).context("Failed to parse config file")
    }

    pub fn verify_config(&self) -> anyhow::Result<()> {
        for token in &self.tokens {
            token.verify_config()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn rejects_token_without_credentials() {
        let config: Config = toml::from_str(
            r#"
                [[tokens]]
                api = "openai"
            "#,
        )
        .unwrap();

        let error = config.verify_config().unwrap_err();
        assert_eq!(
            error.to_string(),
            "At least one of 'raw', 'sha256', or 'sha512' is required."
        );
    }

    #[test]
    fn validates_every_token_entry() {
        let config: Config = toml::from_str(
            r#"
                [[tokens]]
                raw = "valid-token"

                [[tokens]]
                expire_at = "2099-01-01T00:00:00Z"
            "#,
        )
        .unwrap();

        assert!(config.verify_config().is_err());
    }
}
