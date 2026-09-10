use crate::config::Config;
use crate::verifier::Verifier;
use crate::verifier_endpoint::verify_auth;
use axum::Router;
use axum::routing::get;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod config;
mod jwt;
mod token;
mod verifier;
mod verifier_endpoint;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(long, help = "Print configuration for Caddy")]
    caddy: bool,

    #[arg(long, help = "Listen host and port", default_value = "127.0.0.1:9731")]
    listen: String,

    #[arg(
        long,
        help = "Path to tokens file",
        default_value = "/etc/llm-auth-verifier/tokens.toml"
    )]
    tokens: String,
}

fn caddy_config(listen: &str) -> String {
    format!("forward_auth {listen} {{\n    uri /verify\n}}")
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.caddy {
        println!("{}", caddy_config(&args.listen));
        return;
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();
    info!("Starting LLM Auth verifier");

    let config = match Config::load(args.tokens) {
        Ok(c) => match Verifier::new(c).await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to initialize verifier: {e}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    };

    let app = Router::new()
        .route("/verify", get(verify_auth))
        .with_state(config);

    let listener = match tokio::net::TcpListener::bind(&args.listen).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind {}: {}", args.listen, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("{e}");
        std::process::exit(1);
    }
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
