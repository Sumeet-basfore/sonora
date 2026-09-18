use clap::{Parser, Subcommand};
use sonora_common::{init_logging, LogConfig, Result};
use sonora_core::{SonoraApp, SonoraConfig};

#[derive(Parser)]
#[command(name = "sonora", about = "Sonora Command-Line Interface", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current Sonora status
    Status,
    /// Rescan audio library
    Scan { path: Option<String> },
    /// Adjust playback volume
    Volume { level: Option<f32> },
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging(&LogConfig::default());
    let cli = Cli::parse();

    let config = SonoraConfig::default();
    let app = SonoraApp::new(config)?;

    match cli.command {
        Some(Commands::Status) => {
            println!("Sonora v0.1.0 Daemon/App initialized successfully.");
            println!("Data directory: {}", app.config().data_dir.display());
        }
        Some(Commands::Scan { path }) => {
            let target = path.unwrap_or_else(|| "default music directories".to_string());
            println!("Library scan requested for: {target}");
        }
        Some(Commands::Volume { level }) => {
            if let Some(vol) = level {
                println!("Volume set to: {vol:.2}");
            } else {
                println!("Current volume: {:.2}", app.config().default_volume);
            }
        }
        None => {
            println!("Sonora CLI v0.1.0 - Use --help for available commands.");
        }
    }

    Ok(())
}
