use crate::jwt::download::JwkDownloader;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct OidcDiscovery {
    pub issuer: String,
    pub jwks_uri: String,
}

impl JwkDownloader<'_> {
    pub(super) async fn load_oidc_discovery(&self) -> anyhow::Result<OidcDiscovery> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            self.issuer.trim_end_matches('/')
        );
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to load OIDC discovery document: {}",
                response.status()
            );
        }
        let discovery: OidcDiscovery = response.json().await?;
        if discovery.issuer != self.issuer {
            anyhow::bail!("OIDC discovery issuer mismatch: {}", discovery.issuer);
        }
        Ok(discovery)
    }
}
