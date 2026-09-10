use crate::jwt::download::JwkDownloader;
use crate::jwt::download::discovery::OidcDiscovery;
use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<JwkContent>,
}

#[derive(Debug, Deserialize)]
struct JwkContent {
    kid: String,
    #[serde(flatten)]
    param: JwkParam,
    #[serde(rename = "use")]
    using: String,
    alg: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kty")]
enum JwkParam {
    RSA { n: String, e: String },
    EC { crv: String, x: String, y: String },
}

pub(crate) struct JwtDecodingKey {
    pub kid: String,
    pub key: DecodingKey,
}

impl JwkParam {
    fn to_decoding_key(&self) -> anyhow::Result<DecodingKey> {
        match self {
            JwkParam::RSA { n, e } => Ok(DecodingKey::from_rsa_components(&n, &e)?),
            JwkParam::EC { x, y, .. } => Ok(DecodingKey::from_ec_components(&x, &y)?),
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
            let decoding_key = jwk.param.to_decoding_key()?;
            decoding_keys.insert(jwk.kid.clone(), decoding_key);
        }
        Ok(decoding_keys)
    }
}
