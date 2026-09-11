use crate::api::{AiApi, ApiPath};
use axum::http::Method;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PredefinedApis {
    apis: Vec<AiApiWithProvider>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    LlamaCpp,
    Ollama,
    LmStudio,
    VLlm,
}

#[derive(Debug, Clone)]
struct AiApiWithProvider {
    providers: HashSet<Provider>,
    api: AiApi,
}

impl AiApiWithProvider {
    fn new(api: AiApi, providers: &[Provider]) -> Self {
        Self {
            providers: providers.iter().cloned().collect(),
            api,
        }
    }
}

impl PredefinedApis {
    pub fn preset_for(&self, provider: Provider) -> Vec<AiApi> {
        self.apis
            .iter()
            .filter(|api| api.providers.contains(&provider))
            .map(|a| a.api.clone())
            .collect()
    }

    pub fn select(&self, names: HashSet<String>) -> Vec<AiApi> {
        self.apis
            .iter()
            .filter(|a| names.contains(a.api.name()))
            .map(|a| a.api.clone())
            .collect()
    }
}

impl Default for PredefinedApis {
    fn default() -> Self {
        Self {
            apis: vec![
                // OpenAI compatible APIs
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/chat-completions",
                        Method::POST,
                        ApiPath::exact("/v1/chat/completions"),
                    ),
                    &[
                        Provider::LlamaCpp,
                        Provider::Ollama,
                        Provider::LmStudio,
                        Provider::VLlm,
                    ],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/chat-completions-batch",
                        Method::POST,
                        ApiPath::exact("/v1/chat/completions/batch"),
                    ),
                    &[Provider::VLlm],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/responses",
                        Method::POST,
                        ApiPath::exact("/v1/responses"),
                    ),
                    &[
                        Provider::LlamaCpp,
                        Provider::Ollama,
                        Provider::LmStudio,
                        Provider::VLlm,
                    ],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/completions",
                        Method::POST,
                        ApiPath::exact("/v1/completions"),
                    ),
                    &[
                        Provider::LlamaCpp,
                        Provider::Ollama,
                        Provider::LmStudio,
                        Provider::VLlm,
                    ],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/embeddings",
                        Method::POST,
                        ApiPath::exact("/v1/embeddings"),
                    ),
                    &[
                        Provider::LlamaCpp,
                        Provider::Ollama,
                        Provider::LmStudio,
                        Provider::VLlm,
                    ],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/models",
                        Method::GET,
                        ApiPath::regex(Regex::new(r#"/v1/models(/[\w\-_]+)?"#).unwrap()),
                    ),
                    &[
                        Provider::LlamaCpp,
                        Provider::Ollama,
                        Provider::LmStudio,
                        Provider::VLlm,
                    ],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/audio-transcriptions",
                        Method::POST,
                        ApiPath::exact("/v1/audio/transcriptions"),
                    ),
                    &[Provider::VLlm],
                ),
                AiApiWithProvider::new(
                    AiApi::new(
                        "openai/audio-translations",
                        Method::POST,
                        ApiPath::exact("/v1/audio/translations"),
                    ),
                    &[Provider::VLlm],
                ),
                // Anthropic compatible API
                AiApiWithProvider::new(
                    AiApi::new(
                        "anthropic/messages",
                        Method::POST,
                        ApiPath::exact("/v1/messages"),
                    ),
                    &[
                        Provider::LlamaCpp,
                        Provider::Ollama,
                        Provider::LmStudio,
                        Provider::VLlm,
                    ],
                ),
            ],
        }
    }
}
