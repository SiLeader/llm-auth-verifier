mod download;
mod duration_deserializer;
mod issuer;
mod key;

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::jwt::duration_deserializer::Dur;
use jsonwebtoken::Algorithm;
pub use key::JwtVerifier;

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    issuer: String,
    audiences: Vec<String>,
    algorithms: HashSet<Algorithm>,
    access_rules: Vec<AccessRule>,
    cache_ttl: Option<Dur>, // Time-to-live for cached keys in seconds
}

#[derive(Debug, Clone, Deserialize)]
struct AccessRule {
    claims: HashMap<String, serde_json::Value>,
    allowed_apis: HashSet<String>,
}

impl AccessRule {
    fn allows(&self, api: &str, claims: &serde_json::Value) -> bool {
        self.allowed_apis.contains(api)
            && self
                .claims
                .iter()
                .all(|(selector, expected)| claim_matches(claims, selector, expected))
    }
}

fn claim_matches(claims: &serde_json::Value, selector: &str, expected: &serde_json::Value) -> bool {
    let actual = if selector.starts_with('/') {
        claims.pointer(selector)
    } else {
        selector
            .split('.')
            .try_fold(claims, |value, segment| value.get(segment))
    };

    actual.is_some_and(|actual| {
        actual == expected
            || actual
                .as_array()
                .is_some_and(|values| !expected.is_array() && values.contains(expected))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn access_rule_matches_scalar_nested_and_array_claims() {
        let claims = json!({
            "sub": "alice",
            "realm": { "role": "developer" },
            "groups": ["users", "llm-admins"]
        });
        let rule = AccessRule {
            claims: HashMap::from([
                ("sub".to_owned(), json!("alice")),
                ("realm.role".to_owned(), json!("developer")),
                ("/groups".to_owned(), json!("llm-admins")),
            ]),
            allowed_apis: HashSet::from(["openai/chat-completions".to_owned()]),
        };

        assert!(rule.allows("openai/chat-completions", &claims));
        assert!(!rule.allows("openai/models", &claims));
    }

    #[test]
    fn parses_oidc_algorithms_and_arbitrary_claim_values() {
        let config: JwtConfig = toml::from_str(
            r#"
            issuer = "https://auth.example.com"
            audiences = ["llm-service"]
            algorithms = ["RS256", "ES256"]

            [[access_rules]]
            allowed_apis = ["openai/chat-completions"]
            claims = { sub = "alice", admin = true, tenant_id = 42 }
            "#,
        )
        .unwrap();

        assert!(config.algorithms.contains(&Algorithm::RS256));
        assert_eq!(config.access_rules[0].claims["admin"], json!(true));
        assert_eq!(config.access_rules[0].claims["tenant_id"], json!(42));
    }
}
