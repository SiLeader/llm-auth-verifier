use crate::api::{ApiPath, Provider};
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
pub(crate) struct PredefinedApis {
    #[arg(short, long, help = "LLM provider name for filtering")]
    provider: Option<LlmProvider>,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum LlmProvider {
    LlamaCpp,
    Ollama,
    LmStudio,
    VLlm,
}

impl From<LlmProvider> for Provider {
    fn from(provider: LlmProvider) -> Self {
        match provider {
            LlmProvider::LlamaCpp => Provider::LlamaCpp,
            LlmProvider::Ollama => Provider::Ollama,
            LlmProvider::LmStudio => Provider::LmStudio,
            LlmProvider::VLlm => Provider::VLlm,
        }
    }
}

impl PredefinedApis {
    pub(super) fn handle(self) {
        let apis = crate::api::predefined::PredefinedApis::default();
        let apis = apis.list(self.provider.map(From::from));

        for (index, api) in apis.into_iter().enumerate() {
            let mut providers = api
                .providers()
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>();
            providers.sort();
            let providers = providers.join(", ");

            let (mode, path) = match api.api().path() {
                ApiPath::Exact(e) => ("", e.as_str()),
                ApiPath::Regex(r) => (" (regex)", r.as_str()),
            };

            if index != 0 {
                println!("\n---");
            }
            println!("name: {}", api.name());
            println!("supported providers: {}", providers);
            println!("api:");
            println!("  method: {}", api.api().method());
            println!("  path: \"{}\"{}", path, mode);
        }
    }
}
