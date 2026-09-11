use crate::api::{AiApi, ApiConfig};
use axum::http::Method;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ApiDetector {
    apis: Vec<AiApi>,
}

impl ApiDetector {
    pub fn try_new(config: ApiConfig) -> anyhow::Result<Self> {
        let apis = config.into_apis()?;

        debug!("Setting API");
        for api in &apis {
            debug!("enabled: {}: {:?}", api.method(), api.path());
        }
        Ok(Self { apis })
    }

    pub fn detect(&self, method: &Method, path: &str) -> Option<&AiApi> {
        self.apis.iter().find(|api| api.matches(method, path))
    }
}
