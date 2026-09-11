use crate::config::ApiType;
use axum::http::Method;
use regex::Regex;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct AiApi {
    name: String,
    method: Method,
    path: ApiPath,
    api_type: Option<ApiType>,
}

#[derive(Debug, Clone)]
pub enum ApiPath {
    Exact(String),
    Regex(Regex),
}

impl AiApi {
    pub fn new(name: &str, method: Method, path: ApiPath) -> Self {
        Self {
            name: name.to_string(),
            method,
            path,
            api_type: None,
        }
    }

    pub fn openai(name: &str, method: Method, path: ApiPath) -> Self {
        Self {
            name: name.to_string(),
            method,
            path,
            api_type: Some(ApiType::OpenAi),
        }
    }

    pub fn anthropic(name: &str, method: Method, path: ApiPath) -> Self {
        Self {
            name: name.to_string(),
            method,
            path,
            api_type: Some(ApiType::Anthropic),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn api_type(&self) -> Option<ApiType> {
        self.api_type
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &ApiPath {
        &self.path
    }

    pub fn matches(&self, method: &Method, path: &str) -> bool {
        debug!(
            "checking {method}: {path} request for {}: {:?}",
            self.method, path
        );
        if self.method != method {
            debug!("method mismatch");
            return false;
        }
        let is_ok = match &self.path {
            ApiPath::Exact(p) => p == path,
            ApiPath::Regex(r) => r.is_match(path),
        };
        if is_ok {
            debug!(
                "matched {method}: {path} request for {}: {:?}",
                self.method, path
            );
        } else {
            debug!(
                "unmatched {method}: {path} request for {}: {:?}",
                self.method, path
            );
        }
        is_ok
    }
}

impl ApiPath {
    pub fn exact(path: &str) -> Self {
        Self::Exact(path.to_string())
    }

    pub fn regex(path: Regex) -> Self {
        Self::Regex(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{AiApi, ApiPath};
    use axum::http::Method;
    use regex::Regex;

    #[test]
    fn exact_path_requires_matching_method_and_path() {
        let api = AiApi::new("test", Method::POST, ApiPath::exact("/v1/chat/completions"));

        assert!(api.matches(&Method::POST, "/v1/chat/completions"));
        assert!(!api.matches(&Method::GET, "/v1/chat/completions"));
        assert!(!api.matches(&Method::POST, "/v1/completions"));
    }

    #[test]
    fn regex_path_requires_matching_method_and_pattern() {
        let api = AiApi::new(
            "test",
            Method::GET,
            ApiPath::regex(Regex::new(r"^/v1/models(?:/[\w_-]+)?$").unwrap()),
        );

        assert!(api.matches(&Method::GET, "/v1/models"));
        assert!(api.matches(&Method::GET, "/v1/models/example-model"));
        assert!(!api.matches(&Method::POST, "/v1/models"));
        assert!(!api.matches(&Method::GET, "/v1/models/example-model/details"));
    }
}
