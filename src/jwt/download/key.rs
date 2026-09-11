use crate::jwt::download::JwkDownloader;
use crate::jwt::download::discovery::OidcDiscovery;
use jsonwebtoken::DecodingKey;
use std::collections::HashMap;
use tracing::warn;

impl JwkDownloader<'_> {
    async fn load_key_from_endpoint(
        &self,
        discovery: OidcDiscovery,
    ) -> anyhow::Result<jsonwebtoken::jwk::JwkSet> {
        let response = self.client.get(&discovery.jwks_uri).send().await?;
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to load JWK set from {}: {}",
                discovery.jwks_uri,
                response.status()
            );
        }
        Ok(response.json().await?)
    }

    pub(super) async fn load_keys(
        &self,
        discovery: OidcDiscovery,
    ) -> anyhow::Result<HashMap<String, DecodingKey>> {
        let jwk_set = self.load_key_from_endpoint(discovery).await?;
        Ok(decoding_keys_from_jwk_set(jwk_set))
    }
}

fn decoding_keys_from_jwk_set(jwk_set: jsonwebtoken::jwk::JwkSet) -> HashMap<String, DecodingKey> {
    let mut decoding_keys = HashMap::new();
    for jwk in jwk_set.keys {
        let Some(kid) = jwk.common.key_id.clone() else {
            warn!("JWK is missing kid. Skipping.");
            continue;
        };
        match DecodingKey::from_jwk(&jwk) {
            Ok(decoding_key) => {
                decoding_keys.insert(kid, decoding_key);
            }
            Err(error) => warn!("Unsupported JWK for kid {}: {}. Skipping.", kid, error),
        }
    }
    decoding_keys
}

#[cfg(test)]
mod tests {
    use super::decoding_keys_from_jwk_set;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, Validation};
    use serde_json::json;

    #[test]
    fn loads_hmac_keys_from_oct_jwks() {
        let jwks = serde_json::from_value(json!({
            "keys": [{
                "kty": "oct",
                "kid": "hmac-key",
                "k": "c2VjcmV0",
                "alg": "HS256"
            }]
        }))
        .unwrap();
        let keys = decoding_keys_from_jwk_set(jwks);
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("hmac-key".to_owned());
        let token = jsonwebtoken::encode(
            &header,
            &json!({ "exp": 4_102_444_800_u64 }),
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();

        jsonwebtoken::decode::<serde_json::Value>(
            &token,
            &keys["hmac-key"],
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();
    }
}
