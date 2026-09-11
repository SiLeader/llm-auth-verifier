use crate::api::ApiConfig;
use crate::jwt::JwtConfig;
use crate::token::TokenConfig;
use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tokens: Vec<TokenConfig>,
    #[serde(default)]
    pub oidc: Vec<JwtConfig>,
    #[serde(default)]
    pub api: Option<ApiConfig>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiType {
    OpenAi,
    Anthropic,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;
        toml::from_str(&content).context("Failed to parse config file")
    }
}
