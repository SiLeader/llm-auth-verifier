use clap::Parser;

#[derive(Debug, Parser)]
pub(crate) struct CaddyConfigArgs {}

impl CaddyConfigArgs {
    pub(super) fn handle(self, listen: &str) {
        println!("{}", caddy_config(listen));
    }
}

fn caddy_config(listen: &str) -> String {
    format!("forward_auth {listen} {{\n    uri /verify\n}}")
}

#[cfg(test)]
mod tests {
    use super::caddy_config;

    #[test]
    fn caddy_config_uses_supported_forward_auth_subdirectives() {
        let config = caddy_config("llm-auth-verifier:9731");

        assert_eq!(
            config,
            "forward_auth llm-auth-verifier:9731 {\n    uri /verify\n}"
        );
        assert!(!config.contains("header_up"));
    }
}
