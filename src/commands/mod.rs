use crate::commands::caddy_config::CaddyConfigArgs;
use crate::commands::nginx_config::NginxConfigArgs;
use crate::commands::traefik_config::TraefikConfigArgs;
use clap::Subcommand;

mod caddy_config;
mod nginx_config;
mod traefik_config;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Subcommand)]
pub(crate) enum SubCommand {
    CaddyConfig(CaddyConfigArgs),
    TraefikConfig(TraefikConfigArgs),
    NginxConfig(NginxConfigArgs),
}

impl SubCommand {
    pub fn handle(self, listen: &str) {
        match self {
            SubCommand::CaddyConfig(c) => {
                c.handle(listen);
            }
            SubCommand::TraefikConfig(c) => {
                c.handle(listen);
            }
            SubCommand::NginxConfig(c) => {
                c.handle(listen);
            }
        }
    }
}
