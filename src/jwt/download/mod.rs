mod discovery;
mod key;

use reqwest::Client;
use std::collections::HashMap;

pub struct JwkDownloader<'a> {
    client: Client,
    issuer: &'a str,
}

impl<'a> JwkDownloader<'a> {
    pub fn new(issuer: &'a str) -> Self {
        Self {
            client: Client::new(),
            issuer: issuer.trim().trim_end_matches('/'),
        }
    }

    pub async fn load(&self) -> anyhow::Result<HashMap<String, jsonwebtoken::DecodingKey>> {
        let discovery = self.load_oidc_discovery().await?;
        if discovery.issuer != self.issuer {
            anyhow::bail!(
                "Issuer mismatch: expected {}, got {}",
                self.issuer,
                discovery.issuer
            );
        }

        let keys = self.load_keys(discovery).await?;
        Ok(keys)
    }
}
