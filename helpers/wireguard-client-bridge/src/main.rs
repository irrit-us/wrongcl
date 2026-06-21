mod config;
mod socks;
mod wireguard;

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

use crate::config::AppConfig;
use crate::wireguard::WireGuardRuntime;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    init_tracing();

    let config_path = parse_config_path()?;
    let config = AppConfig::load(&config_path)?;
    let runtime = Arc::new(WireGuardRuntime::start(config.clone()).await?);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        tokio::select! {
            _ = wait_for_stdin_close() => {}
            _ = wait_for_signal() => {}
        }
        let _ = shutdown_tx.send(true);
    });

    socks::run_socks_server(config.listen, runtime, shutdown_rx).await
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

fn parse_config_path() -> Result<PathBuf> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--config"), Some(path), None) => Ok(PathBuf::from(path)),
        _ => bail!("usage: wireguard-client-bridge --config /path/to/config.json"),
    }
}

async fn wait_for_stdin_close() {
    let _ = tokio::task::spawn_blocking(|| {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 4096];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    })
    .await
    .context("stdin watcher task failed");
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = match signal(SignalKind::terminate()) {
        Ok(signal) => signal,
        Err(_) => {
            let _ = tokio::signal::ctrl_c().await;
            return;
        }
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
