mod config;
mod server;
mod tls;

use clap::Parser;
use config::Config;
use std::future;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use tokio::{fs, signal, time};
use tracing::subscriber;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use rkvm_input::clipsync::ClipSyncOptions;

#[derive(Parser)]
#[structopt(name = "rkvm-server", about = "The rkvm server application")]
struct Args {
    #[structopt(help = "Path to configuration file")]
    config_path: PathBuf,
    #[structopt(help = "Shutdown after N seconds", long, short)]
    shutdown_after: Option<u64>,
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

    let acceptor = match tls::configure(&config.certificate, &config.key).await {
        Ok(acceptor) => acceptor,
        Err(err) => {
            tracing::error!("Error configuring TLS: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let shutdown = async {
        match args.shutdown_after {
            Some(shutdown_after) => time::sleep(Duration::from_secs(shutdown_after)).await,
            None => future::pending().await,
        }
    };

    let switch_keys = config.switch_keys.into_iter().map(Into::into).collect();
    let propagate_switch_keys = config.propagate_switch_keys.unwrap_or(true);



    let clipsync_opts = ClipSyncOptions {
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
        result = server::run(config.listen, acceptor, &config.password, &switch_keys, propagate_switch_keys, &clipsync_opts) => {
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
        _ = shutdown => {
            tracing::info!("Shutting down as requested");
        }
    }

    ExitCode::SUCCESS
}
