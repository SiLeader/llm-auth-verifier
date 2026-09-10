pub use crate::config::token::*;
use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

mod token;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub tokens: Vec<Token>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;
        toml::from_str(&content).context("Failed to parse config file")
    }
}
