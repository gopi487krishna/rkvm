mod client;
mod config;
mod tls;

use clap::Parser;
use config::Config;
use std::path::PathBuf;
use std::process::ExitCode;
use tokio::{fs, signal};
use tracing::subscriber;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use rkvm_input::clipsync;

#[derive(Parser)]
#[structopt(name = "rkvm-client", about = "The rkvm client application")]
struct Args {
    #[clap(help = "Path to configuration file")]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().without_time());

    subscriber::set_global_default(registry).unwrap();

    let args = Args::parse();
    let config = match fs::read_to_string(&args.config_path).await {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Error reading config: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let config = match toml::from_str::<Config>(&config) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Error parsing config: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let connector = match tls::configure(&config.certificate).await {
        Ok(connector) => connector,
        Err(err) => {
            tracing::error!("Error configuring TLS: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let clipsync_opts = clipsync::ClipSyncOptions {
        clip_sync_enabled: config.clipsync_enabled.unwrap_or(false),
        sync_provider_path: config.sync_provider_path.unwrap_or(String::new()).clone(),
        xdg_runtime_dir: config.xdg_runtime_dir.unwrap_or(String::new()).clone(),
        wayland_display: config.wayland_display.unwrap_or(String::new()).clone(),
        piknik_config_path: config.piknik_config_path.unwrap_or(String::new()).clone(),
        piknik_bin_name: config.piknik_bin_name.unwrap_or(String::new()).clone(),
        piknik_path: config.piknik_path.unwrap_or(String::new()).clone(),
        uid : config.uid.unwrap_or(u32::MAX),
        gid : config.gid.unwrap_or(u32::MAX),
    };

    tokio::select! {
        result = client::run(&config.server.hostname, config.server.port, connector, &config.password, &clipsync_opts) => {
            if let Err(err) = result {
                tracing::error!("Error: {}", err);
                return ExitCode::FAILURE;
            }
        }
        // This is needed to properly clean libevdev stuff up.
        result = signal::ctrl_c() => {
            if let Err(err) = result {
                tracing::error!("Error setting up signal handler: {}", err);
                return ExitCode::FAILURE;
            }

            tracing::info!("Exiting on signal");
        }
    }

    ExitCode::SUCCESS
}
