use crate::config::Config;
use crate::verifier::verify_auth;
use axum::Router;
use axum::routing::get;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod config;
mod verifier;

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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.caddy {
        println!("forward_auth {} {{", args.listen);
        println!("    uri /verify");
        println!("}}");
        return;
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();
    info!("Starting LLM Auth verifier");

    let config = match Config::load(args.tokens) {
        Ok(c) => {
            if let Err(e) = c.verify_config() {
                error!("Invalid configuration: {e}");
                std::process::exit(1);
            }
            c
        }
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
