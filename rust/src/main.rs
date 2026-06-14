use std::path::PathBuf;
use std::sync::mpsc;

use clap::{Args, Parser, Subcommand};
use serde_json::json;
use wrongcl_native::client::RawVlessTcpClient;
use wrongcl_native::config::{config_example, default_config, ClientConfig};
use wrongcl_native::manager::ConnectionManager;
use wrongcl_native::protocol::Target;
use wrongcl_native::Result;

#[derive(Debug, Parser)]
#[command(name = "wrongcl-headless")]
#[command(about = "Headless wrongsv raw VLESS TCP client")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(ServeArgs),
    Probe(ProbeArgs),
    ConfigExample,
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    server_host: Option<String>,
    #[arg(long)]
    server_port: Option<u16>,
    #[arg(long)]
    uuid: Option<String>,
    #[arg(long)]
    listen_host: Option<String>,
    #[arg(long)]
    listen_port: Option<u16>,
}

#[derive(Debug, Args)]
struct ProbeArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    server_host: Option<String>,
    #[arg(long)]
    server_port: Option<u16>,
    #[arg(long)]
    uuid: Option<String>,
    #[arg(long)]
    target_host: String,
    #[arg(long)]
    target_port: u16,
    #[arg(long, default_value = "")]
    payload: String,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Serve(args) => serve(args),
        Command::Probe(args) => probe(args),
        Command::ConfigExample => {
            print!("{}", config_example());
            Ok(())
        }
    }
}

fn serve(args: ServeArgs) -> Result<()> {
    let config = resolve_config(
        args.config,
        args.server_host,
        args.server_port,
        args.uuid,
        args.listen_host,
        args.listen_port,
    )?;
    let manager = ConnectionManager::new();
    let snapshot = manager.start_proxy(config)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "event": "started",
            "proxy": snapshot,
        }))?
    );

    let (tx, rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .map_err(|e| wrongcl_native::ClientError::Config(format!("install Ctrl-C handler: {e}")))?;

    let _ = rx.recv();
    let snapshot = manager.stop_proxy()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "event": "stopped",
            "proxy": snapshot,
        }))?
    );
    Ok(())
}

fn probe(args: ProbeArgs) -> Result<()> {
    let config = resolve_config(
        args.config,
        args.server_host,
        args.server_port,
        args.uuid,
        None,
        None,
    )?;
    let target = Target::new(args.target_host, args.target_port)?;
    let client = RawVlessTcpClient::new(config.server)?;
    let result = client.probe(&target, &args.payload)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "probe": result,
        }))?
    );
    Ok(())
}

fn resolve_config(
    path: Option<PathBuf>,
    server_host: Option<String>,
    server_port: Option<u16>,
    uuid: Option<String>,
    listen_host: Option<String>,
    listen_port: Option<u16>,
) -> Result<ClientConfig> {
    let mut config = match path {
        Some(path) => ClientConfig::from_file(path)?,
        None => default_config(),
    };

    if let Some(value) = server_host {
        config.server.host = value;
    }
    if let Some(value) = server_port {
        config.server.port = value;
    }
    if let Some(value) = uuid {
        config.server.uuid = value;
    }
    if let Some(value) = listen_host {
        config.local.host = value;
    }
    if let Some(value) = listen_port {
        config.local.port = value;
    }

    config.validate()?;
    Ok(config)
}
