mod api;
mod config;
mod db;
mod error;
mod models;
mod naming;
mod pipeline;
mod watcher;

use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "subsforge", version, about = "Auto-generate and translate subtitles")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server and watcher daemon
    Serve {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
    },
    /// Process a single media file
    Process {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        /// Path to the media file
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "subsforge=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config: config_path } => cmd_serve(config_path).await,
        Commands::Process { config: config_path, file } => cmd_process(config_path, file).await,
    }
}

async fn cmd_serve(config_path: PathBuf) -> anyhow::Result<()> {
    let config = Arc::new(config::Config::load(&config_path)?);
    let pool = db::init_pool(&config.database.url).await?;
    let client = reqwest::Client::new();

    let state = api::AppState {
        config: config.clone(),
        pool: pool.clone(),
        client: client.clone(),
    };

    // Start watcher in background
    let watcher_config = config.clone();
    let watcher_pool = pool.clone();
    let watcher_client = client.clone();
    tokio::spawn(async move {
        watcher::start(watcher_config, watcher_pool, watcher_client).await;
    });

    // Start API server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr, "subsforge server started");

    axum::serve(listener, api::router(state)).await?;

    Ok(())
}

async fn cmd_process(config_path: PathBuf, file: PathBuf) -> anyhow::Result<()> {
    let config = Arc::new(config::Config::load(&config_path)?);
    let pool = db::init_pool(&config.database.url).await?;
    let client = reqwest::Client::new();

    let file = file.canonicalize()?;
    let title = file.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let job_id = db::create_job(
        &pool,
        &file.to_string_lossy(),
        "manual",
        None,
        Some(&title),
        &config.general.target_languages,
    ).await?;

    info!(file = %file.display(), job_id, "processing file");
    pipeline::process_job(&config, &pool, &client, job_id, &file).await?;
    info!(job_id, "done");

    Ok(())
}
