#![allow(clippy::collapsible_if)]

use std::path::PathBuf;
use std::sync::mpsc;

use clap::{Args, Parser, Subcommand};
use serde_json::json;
use wrongcl_native::Result;
use wrongcl_native::adapter::{adapt_wrongsv_config, inspect_wrongsv_config};
use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, config_example, default_config};
use wrongcl_native::endpoint::ProxyProtocol;
use wrongcl_native::manager::ConnectionManager;
use wrongcl_native::protocol::Target;

#[derive(Debug, Parser)]
#[command(name = "wrongcl-headless")]
#[command(
    about = "Headless wrongsv client (VLESS / Trojan / Mixed over raw / WebSocket / HTTPUpgrade, with optional TLS)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(ServeArgs),
    Probe(ProbeArgs),
    Capabilities(CapabilityArgs),
    Adapt(AdaptArgs),
    Stack(StackArgs),
    TunStatus,
    ConfigExample,
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    wrongsv_config: Option<PathBuf>,
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
    wrongsv_config: Option<PathBuf>,
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

#[derive(Debug, Args)]
struct CapabilityArgs {
    #[arg(long)]
    wrongsv_config: PathBuf,
}

#[derive(Debug, Args)]
struct AdaptArgs {
    #[arg(long)]
    wrongsv_config: PathBuf,
    #[arg(long)]
    server_host: String,
    #[arg(long, default_value = "127.0.0.1")]
    listen_host: String,
    #[arg(long, default_value_t = 1080)]
    listen_port: u16,
}

#[derive(Debug, Args)]
struct StackArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    wrongsv_config: Option<PathBuf>,
    #[arg(long)]
    server_host: Option<String>,
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
        Command::Capabilities(args) => capabilities(args),
        Command::Adapt(args) => adapt(args),
        Command::Stack(args) => stack(args),
        Command::TunStatus => tun_status(),
        Command::ConfigExample => {
            print!("{}", config_example());
            Ok(())
        }
    }
}

fn serve(args: ServeArgs) -> Result<()> {
    let config = resolve_config(
        args.config,
        args.wrongsv_config,
        args.server_host,
        args.server_port,
        args.uuid,
        args.listen_host,
        args.listen_port,
    )?;
    let stack_summary = config
        .resolve_active_endpoint()?
        .server
        .endpoint
        .stack_summary();
    let manager = ConnectionManager::new();
    let snapshot = manager.start_proxy(config)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "event": "started",
            "stack": stack_summary,
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
        args.wrongsv_config,
        args.server_host,
        args.server_port,
        args.uuid,
        None,
        None,
    )?;
    let target = Target::new(args.target_host, args.target_port)?;
    let server = config.resolve_active_endpoint()?.server.clone();
    let client = WrongsvClient::new(server)?;
    let stack = client.stack_summary();
    let result = client.probe(&target, &args.payload)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "stack": stack,
            "probe": result,
        }))?
    );
    Ok(())
}

fn capabilities(args: CapabilityArgs) -> Result<()> {
    let report = inspect_wrongsv_config(args.wrongsv_config)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn adapt(args: AdaptArgs) -> Result<()> {
    let adapted = adapt_wrongsv_config(
        args.wrongsv_config,
        args.server_host,
        args.listen_host,
        args.listen_port,
    )?;
    println!("{}", serde_json::to_string_pretty(&adapted)?);
    Ok(())
}

fn stack(args: StackArgs) -> Result<()> {
    let config = resolve_config(
        args.config,
        args.wrongsv_config,
        args.server_host,
        None,
        None,
        None,
        None,
    )?;
    let ep = config.resolve_active_endpoint()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "stack": ep.server.endpoint.stack_summary(),
            "proxy": ep.server.endpoint.proxy.id(),
            "transport": ep.server.endpoint.transport.id(),
            "outer_security": ep.server.endpoint.outer_security.id(),
        }))?
    );
    Ok(())
}

fn tun_status() -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&wrongcl_native::current_tun_status())?
    );
    Ok(())
}

fn resolve_config(
    path: Option<PathBuf>,
    wrongsv_path: Option<PathBuf>,
    server_host: Option<String>,
    server_port: Option<u16>,
    uuid: Option<String>,
    listen_host: Option<String>,
    listen_port: Option<u16>,
) -> Result<ClientConfig> {
    if path.is_some() && wrongsv_path.is_some() {
        return Err(wrongcl_native::ClientError::Config(
            "use either --config or --wrongsv-config, not both".into(),
        ));
    }

    let mut config = match (path, wrongsv_path) {
        (Some(path), None) => ClientConfig::from_file(path)?,
        (None, Some(path)) => {
            let adapted = adapt_wrongsv_config(
                path,
                server_host.clone().unwrap_or_else(|| "127.0.0.1".into()),
                listen_host.clone().unwrap_or_else(|| "127.0.0.1".into()),
                listen_port.unwrap_or(1080),
            )?;
            let document = adapted.config.ok_or_else(|| {
                wrongcl_native::ClientError::UnsupportedProtocol(format!(
                    "wrongsv active profile '{}' is not runnable in wrongcl yet: {}",
                    adapted.report.active_profile, adapted.report.active_reason
                ))
            })?;
            let document_json = serde_json::to_string(&document)?;
            ClientConfig::from_legacy_document_json(&document_json)?
        }
        (None, None) => default_config(),
        (Some(_), Some(_)) => unreachable!("checked above"),
    };

    let active_name = match &config.active {
        wrongcl_native::ActiveSelection::Endpoint { name } => name.clone(),
        wrongcl_native::ActiveSelection::Group { name } => {
            config.find_group(name)?.selected.clone().ok_or_else(|| {
                wrongcl_native::ClientError::Config(format!(
                    "group '{name}' has no selected member"
                ))
            })?
        }
    };
    let ep = config
        .endpoints
        .iter_mut()
        .find(|e| e.name == active_name)
        .ok_or_else(|| {
            wrongcl_native::ClientError::Config(format!("endpoint '{active_name}' is not defined"))
        })?;
    if let Some(value) = server_host {
        ep.server.host = value;
    }
    if let Some(value) = server_port {
        ep.server.port = value;
    }
    if let Some(value) = uuid {
        if let ProxyProtocol::Vless(opts) = &mut ep.server.endpoint.proxy {
            opts.uuid = value;
        } else {
            return Err(wrongcl_native::ClientError::Config(
                "--uuid only applies to a VLESS proxy".into(),
            ));
        }
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
