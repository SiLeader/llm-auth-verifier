use crate::jwt::download::JwkDownloader;
use crate::jwt::download::discovery::OidcDiscovery;
use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<JwkContent>,
}

#[derive(Debug, Deserialize)]
struct JwkContent {
    kid: String,
    #[serde(flatten)]
    param: JwkParam,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize)]
#[serde(tag = "kty")]
enum JwkParam {
    RSA {
        n: String,
        e: String,
    },
    EC {
        x: String,
        y: String,
    },
    #[serde(other)]
    Unsupported,
}

impl JwkParam {
    fn to_decoding_key(&self) -> anyhow::Result<Option<DecodingKey>> {
        match self {
            JwkParam::RSA { n, e } => Ok(Some(DecodingKey::from_rsa_components(n, e)?)),
            JwkParam::EC { x, y } => Ok(Some(DecodingKey::from_ec_components(x, y)?)),
            JwkParam::Unsupported => Ok(None),
        }
    }
}

impl JwkDownloader<'_> {
    async fn load_key_from_endpoint(
        &self,
        discovery: OidcDiscovery,
    ) -> anyhow::Result<Vec<JwkContent>> {
        let response = self.client.get(&discovery.jwks_uri).send().await?;
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to load JWK set from {}: {}",
                discovery.jwks_uri,
                response.status()
            );
        }
        let jwk_set: JwkSet = response.json().await?;
        Ok(jwk_set.keys)
    }

    pub(super) async fn load_keys(
        &self,
        discovery: OidcDiscovery,
    ) -> anyhow::Result<HashMap<String, DecodingKey>> {
        let keys = self.load_key_from_endpoint(discovery).await?;
        let mut decoding_keys = HashMap::new();
        for jwk in keys {
            let Some(decoding_key) = jwk.param.to_decoding_key()? else {
                warn!("Unsupported JWK type for kid {}. Skipping.", jwk.kid);
                continue;
            };
            decoding_keys.insert(jwk.kid.clone(), decoding_key);
        }
        Ok(decoding_keys)
    }
}
