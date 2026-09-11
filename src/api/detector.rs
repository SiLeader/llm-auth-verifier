use crate::api::{AiApi, ApiConfig};
use axum::http::Method;

#[derive(Debug, Clone)]
pub struct ApiDetector {
    apis: Vec<AiApi>,
}

impl ApiDetector {
    pub fn try_new(config: ApiConfig) -> anyhow::Result<Self> {
        Ok(Self {
            apis: config.into_apis()?,
        })
    }

    pub fn detect(&self, method: &Method, path: &str) -> Option<&AiApi> {
        for api in &self.apis {
            if api.matches(method, path) {
                return Some(&api);
            }
        }
        None
    }
}
