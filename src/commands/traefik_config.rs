use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
pub(crate) struct TraefikConfigArgs {
    #[arg(short, long, help = "Configuration format", default_value = "yaml")]
    format: Format,

    #[arg(
        short,
        long,
        help = "Name of middleware",
        default_value = "llm-auth-verifier"
    )]
    name: String,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum Format {
    Yaml,
    Toml,
    Labels,
    Tags,
    Kubernetes,
}

impl TraefikConfigArgs {
    pub(super) fn handle(self, listen: &str) {
        let name = self.name;
        let address = format!("http://{listen}/verify");
        match self.format {
            Format::Yaml => {
                println!(
                    "http:
  middlewares:
    {name}:
      forwardAuth:
        address: \"{address}\""
                );
            }
            Format::Toml => {
                println!(
                    "[http.middlewares]
  [http.middlewares.{name}.forwardAuth]
    address = \"{address}\""
                )
            }
            Format::Labels => {
                println!(
                    "labels:
  - \"traefik.http.middlewares.{name}.forwardauth.address={address}\""
                )
            }
            Format::Tags => {
                println!(
                    "{{\"Tags\":[\"traefik.http.middlewares.{name}.forwardauth.address={address}\"]}}"
                );
            }
            Format::Kubernetes => {
                println!(
                    "apiVersion: traefik.io/v1alpha1
kind: Middleware
metadata:
  name: {name}
spec:
  forwardAuth:
    address: \"{address}\""
                )
            }
        }
    }
}
