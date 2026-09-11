use crate::api::AiApi;
use crate::api::predefined::{PredefinedApis, Provider};
use std::collections::HashSet;

#[derive(Debug, Default, Clone)]
pub struct ApiBuilder {
    predefined: PredefinedApis,
    base_provider: Option<Provider>,
    api_names: HashSet<String>,
    apis: Vec<AiApi>,
}

impl ApiBuilder {
    pub fn set_base(&mut self, provider: Provider) {
        self.base_provider = Some(provider);
    }

    pub fn use_api_named(&mut self, api_names: Vec<String>) {
        self.api_names.extend(api_names);
    }

    pub fn use_api(&mut self, api: AiApi) {
        self.apis.push(api);
    }

    pub fn build(self) -> Vec<AiApi> {
        let mut api = if let Some(provider) = self.base_provider {
            self.predefined.preset_for(provider)
        } else {
            vec![]
        };
        api.extend(self.apis);
        api.extend(self.predefined.select(self.api_names));

        api
    }
}
