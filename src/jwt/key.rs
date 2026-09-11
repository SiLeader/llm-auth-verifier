use crate::api::AiApi;
use crate::jwt::JwtConfig;
use crate::jwt::download::JwkDownloader;
use crate::jwt::issuer::JwkIssuer;
use tracing::info;

#[derive(Debug, Clone)]
pub struct JwtVerifier {
    jwks: Vec<JwkIssuer>,
}

impl JwtVerifier {
    pub async fn load(config: Vec<JwtConfig>) -> anyhow::Result<Self> {
        let mut jwks = Vec::with_capacity(config.len());
        for cfg in config {
            let downloader = JwkDownloader::new(&cfg.issuer)?;
            let keys = downloader.load().await?;
            let iss = JwkIssuer::new(cfg, keys);
            jwks.push(iss);
        }
        Ok(Self { jwks })
    }

    pub async fn verify(&self, api: &AiApi, token: &str) -> anyhow::Result<()> {
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow::anyhow!("Token missing 'kid' in header"))?;

        let mut last_error = None;
        for issuer in &self.jwks {
            match issuer.verify(api, header.alg, &kid, token).await {
                Ok(true) => {
                    return Ok(());
                }
                Ok(false) => {}
                Err(error) => last_error = Some(error),
            }
        }

        info!(
            audit=true,
            auth_type="oidc",
            allowed=false,
            api_name=%api.name()
        );

        if let Some(error) = last_error {
            return Err(error);
        }

        anyhow::bail!("No matching key found for 'kid': {}", kid)
    }
}
