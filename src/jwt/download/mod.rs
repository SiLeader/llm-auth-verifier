mod discovery;
mod key;

use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct JwkDownloader<'a> {
    client: Client,
    issuer: &'a str,
}

impl<'a> JwkDownloader<'a> {
    pub fn new(issuer: &'a str) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                // Tests serve discovery and JWKS from a local plain-HTTP server.
                .https_only(!cfg!(test))
                .build()?,
            issuer,
        })
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
