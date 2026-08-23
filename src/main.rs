mod amatsukaze;
mod api;
mod bridge;
mod config;
mod epgstation;
#[cfg(windows)]
mod gui;
mod support;

use std::{env, future::Future, path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use bridge::BridgeService;
use config::Config;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let options = Options::from_env()?;

    #[cfg(windows)]
    if !options.cli {
        let logs = gui::init_logging();
        let config = Config::load(&options.config_path)?;
        gui::run(config, logs)?;
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::load(&options.config_path)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?;
    runtime.block_on(run_server(config, shutdown_signal()))
}

pub(crate) async fn run_server<F>(config: Config, shutdown: F) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let listen = config.listen;
    let service = Arc::new(BridgeService::new(config)?);
    let app = api::router(service);
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind {listen}"))?;

    info!(%listen, "bridge started");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .context("HTTP server failed")?;
    Ok(())
}

struct Options {
    config_path: PathBuf,
    cli: bool,
}

impl Options {
    fn from_env() -> Result<Self> {
        let mut config_path = None;
        let mut cli = false;

        for argument in env::args_os().skip(1) {
            if argument == "--cli" {
                cli = true;
            } else if argument.to_string_lossy().starts_with('-') {
                bail!("unknown option: {}", argument.to_string_lossy());
            } else if config_path.replace(PathBuf::from(&argument)).is_some() {
                bail!("only one config path can be specified");
            }
        }

        Ok(Self {
            config_path: config_path.unwrap_or_else(|| PathBuf::from("config.toml")),
            cli,
        })
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
