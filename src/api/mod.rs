use crate::api::builder::ApiBuilder;
pub use crate::api::predefined::Provider;
use axum::http::Method;
use regex::Regex;
use serde::Deserialize;
use std::str::FromStr;

mod api;
mod builder;
mod detector;
mod predefined;

pub use api::*;
pub use detector::*;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ApiConfig {
    provider: Option<Provider>,
    custom_apis: Vec<CustomApi>,
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
