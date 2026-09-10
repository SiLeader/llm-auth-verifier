use crate::jwt::JwtConfig;
use crate::jwt::download::JwkDownloader;
use crate::jwt::issuer::JwkIssuer;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct JwtVerifier {
    jwks: Vec<JwkIssuer>,
}

impl JwtVerifier {
    pub async fn load(config: Vec<JwtConfig>) -> anyhow::Result<Self> {
        let mut jwks = Vec::with_capacity(config.len());
        for cfg in config {
            let downloader = JwkDownloader::new(&cfg.issuer);
            let keys = downloader.load().await?;
            let iss = JwkIssuer::new(cfg, keys);
            jwks.push(iss);
        }
        Ok(Self { jwks })
    }

    pub async fn verify(&self, token: &str) -> anyhow::Result<()> {
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow::anyhow!("Token missing 'kid' in header"))?;

        for iss in &self.jwks {
            if iss.verify(header.alg, &kid, token).await? {
                return Ok(());
            }
        }

        self.refresh_keys().await;

        anyhow::bail!("No matching key found for 'kid': {}", kid)
    }

    pub async fn refresh_keys(&self) {
        for iss in &self.jwks {
            if let Err(e) = iss.refresh_keys().await {
                warn!("Failed to refresh keys for issuer {}: {}", iss.issuer(), e);
            }
        }
    }
}
