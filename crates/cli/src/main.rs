mod api_client;
mod commands;
mod config;
mod upload;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "statichost", version, about = "Self-hosted static site + tunnel CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Save server URL and token to ~/.statichost/config.toml
    Login {
        #[arg(long)]
        host: String,
        #[arg(long)]
        token: String,
    },
    /// Deploy a static site folder
    Deploy {
        path: PathBuf,
        #[arg(long)]
        name: String,
    },
    /// Run `flutter build web` then deploy ./build/web
    DeployFlutter {
        #[arg(long)]
        name: String,
    },
    /// List deployed sites
    List,
    /// Delete a deployed site
    Delete {
        name: String,
        #[arg(long)]
        force: bool,
    },
    /// Expose a local port over a public subdomain
    Tunnel {
        port: u16,
        #[arg(long)]
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,statichost=info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Login { host, token } => commands::login::run(host, token).await,
        Cmd::Deploy { path, name } => commands::deploy::run(path, name).await,
        Cmd::DeployFlutter { name } => commands::deploy_flutter::run(name).await,
        Cmd::List => commands::list::run().await,
        Cmd::Delete { name, force } => commands::delete::run(name, force).await,
        Cmd::Tunnel { port, name } => commands::tunnel::run(port, name).await,
    }
}
