use clap::Parser;

#[derive(Debug, Parser)]
pub(crate) struct NginxConfigArgs;

impl NginxConfigArgs {
    pub(super) fn handle(self, listen: &str) {
        let proxy = format!("http://{listen}/verify");
        println!(
            "location = /__/llm-auth-verifier/verify {{
    internal;
    proxy_pass {proxy};
    proxy_set_header X-Forwarded-Uri $request_uri;
    proxy_set_header X-Forwarded-Method $request_method;
}}"
        );
    }
}
