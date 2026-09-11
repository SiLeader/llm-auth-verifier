use crate::api::builder::ApiBuilder;
pub use crate::api::predefined::Provider;
use axum::http::Method;
use regex::Regex;
use serde::Deserialize;
use std::str::FromStr;

mod builder;
#[path = "api.rs"]
mod definition;
mod detector;
mod predefined;

pub use definition::*;
pub use detector::*;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    provider: Option<Provider>,
    #[serde(default)]
    custom_apis: Vec<CustomApi>,
    #[serde(default)]
    named_apis: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CustomApi {
    name: Option<String>,
    method: String,
    path: String,
    path_type: PathType,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PathType {
    Exact,
    Regex,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider: Some(Provider::LlamaCpp),
            custom_apis: vec![],
            named_apis: vec![],
        }
    }
}

impl ApiConfig {
    #[cfg(test)]
    pub fn for_provider(provider: Provider) -> ApiConfig {
        Self {
            provider: Some(provider),
            ..Default::default()
        }
    }
    fn into_apis(self) -> anyhow::Result<Vec<AiApi>> {
        let mut builder = ApiBuilder::default();
        if let Some(provider) = self.provider {
            builder.set_base(provider);
        }
        for ca in self.custom_apis {
            let name = ca
                .name
                .unwrap_or_else(|| format!("{}:{}", ca.method, ca.path));

            let api = AiApi::new(
                &name,
                Method::from_str(&ca.method)?,
                match ca.path_type {
                    PathType::Exact => ApiPath::Exact(ca.path),
                    PathType::Regex => ApiPath::Regex(Regex::new(&ca.path)?),
                },
            );
            builder.use_api(api);
        }
        builder.use_api_named(self.named_apis);
        Ok(builder.build())
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiConfig, ApiDetector};
    use crate::config::ApiType;
    use axum::http::Method;

    fn detector(config: &str) -> ApiDetector {
        let config = toml::from_str::<ApiConfig>(config).unwrap();
        ApiDetector::try_new(config).unwrap()
    }

    #[test]
    fn provider_selects_supported_typed_apis() {
        let detector = detector(r#"provider = "llama-cpp""#);

        let openai = detector
            .detect(&Method::POST, "/v1/chat/completions")
            .unwrap();
        let anthropic = detector.detect(&Method::POST, "/v1/messages").unwrap();

        assert_eq!(openai.name(), "openai/chat-completions");
        assert_eq!(openai.api_type(), Some(ApiType::OpenAi));
        assert_eq!(anthropic.name(), "anthropic/messages");
        assert_eq!(anthropic.api_type(), Some(ApiType::Anthropic));
        assert!(
            detector
                .detect(&Method::POST, "/v1/audio/transcriptions")
                .is_none()
        );
    }

    #[test]
    fn vllm_provider_includes_vllm_only_apis() {
        let detector = detector(r#"provider = "v-llm""#);

        assert!(
            detector
                .detect(&Method::POST, "/v1/audio/transcriptions")
                .is_some()
        );
    }

    #[test]
    fn named_api_can_be_enabled_without_provider() {
        let detector = detector(r#"named_apis = ["openai/chat-completions"]"#);

        let api = detector
            .detect(&Method::POST, "/v1/chat/completions")
            .unwrap();
        assert_eq!(api.name(), "openai/chat-completions");
        assert_eq!(api.api_type(), Some(ApiType::OpenAi));
        assert!(detector.detect(&Method::POST, "/v1/responses").is_none());
    }

    #[test]
    fn custom_exact_and_regex_apis_are_detected() {
        let detector = detector(
            r#"
            [[custom_apis]]
            name = "custom/exact"
            method = "POST"
            path = "/custom"
            path_type = "exact"

            [[custom_apis]]
            method = "GET"
            path = "^/models/[0-9]+$"
            path_type = "regex"
            "#,
        );

        assert_eq!(
            detector.detect(&Method::POST, "/custom").unwrap().name(),
            "custom/exact"
        );
        assert_eq!(
            detector.detect(&Method::GET, "/models/42").unwrap().name(),
            "GET:^/models/[0-9]+$"
        );
        assert!(detector.detect(&Method::GET, "/models/latest").is_none());
    }

    #[test]
    fn invalid_custom_api_method_or_regex_is_rejected() {
        let invalid_method = toml::from_str::<ApiConfig>(
            r#"
            [[custom_apis]]
            method = "NOT A METHOD"
            path = "/custom"
            path_type = "exact"
            "#,
        )
        .unwrap();
        let invalid_regex = toml::from_str::<ApiConfig>(
            r#"
            [[custom_apis]]
            method = "GET"
            path = "["
            path_type = "regex"
            "#,
        )
        .unwrap();

        assert!(ApiDetector::try_new(invalid_method).is_err());
        assert!(ApiDetector::try_new(invalid_regex).is_err());
    }
}
