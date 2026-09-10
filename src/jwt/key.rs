use crate::jwt::JwtConfig;
use crate::jwt::download::JwkDownloader;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct JwtVerifier {
    jwks: Vec<JwkIssuer>,
}

#[derive(Debug, Clone)]
struct JwkIssuer {
    config: JwtConfig,
    keys: HashMap<String, jsonwebtoken::DecodingKey>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    aud: String,
    iss: String,
    exp: usize,
}

impl JwtVerifier {
    pub async fn load(config: &[JwtConfig]) -> anyhow::Result<Self> {
        let mut jwks = Vec::with_capacity(config.len());
        for cfg in config {
            let downloader = JwkDownloader::new(&cfg.issuer);
            let keys = downloader.load().await?;
            let iss = JwkIssuer {
                config: cfg.clone(),
                keys,
            };
            jwks.push(iss);
        }
        Ok(Self { jwks })
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<()> {
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow::anyhow!("Token missing 'kid' in header"))?;

        for iss in &self.jwks {
            if let Some(decoding_key) = iss.keys.get(&kid) {
                let validation = jsonwebtoken::Validation::new(header.alg);
                let claims = jsonwebtoken::decode::<Claims>(token, decoding_key, &validation)
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Token verification failed for issuer {}: {}",
                            iss.config.issuer,
                            e
                        )
                    })?;

                if claims.claims.iss != iss.config.issuer
                    || claims.claims.aud != iss.config.audience
                    || !iss.config.subjects.contains(&claims.claims.sub)
                {
                    anyhow::bail!(
                        "Token claims do not match expected values for issuer {}",
                        iss.config.issuer
                    );
                }

                return Ok(());
            }
        }

        anyhow::bail!("No matching key found for 'kid': {}", kid)
    }
}
