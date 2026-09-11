use crate::config::ApiType;
use axum::http::Method;
use regex::Regex;

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

    pub fn matches(&self, method: &Method, path: &str) -> bool {
        if self.method != method {
            return false;
        }
        match &self.path {
            ApiPath::Exact(p) => p.as_str() == method,
            ApiPath::Regex(r) => r.is_match(path),
        }
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
