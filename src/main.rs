use check::get_updates;
use chrono::Local;
use clap::{Parser, Subcommand};
use docker::{get_image_from_docker_daemon, get_images_from_docker_daemon};
#[cfg(feature = "cli")]
use formatting::{print_raw_updates, print_updates, Spinner};
#[cfg(feature = "server")]
use server::serve;
use std::path::PathBuf;
use utils::{load_config, CliConfig};

pub mod check;
pub mod docker;
#[cfg(feature = "cli")]
pub mod formatting;
pub mod image;
pub mod registry;
#[cfg(feature = "server")]
pub mod server;
pub mod utils;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = None)]
    socket: Option<String>,
    #[arg(short, long, default_value_t = String::new(), help = "Config file path")]
    config_path: String,
    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Enable verbose (debug) logging"
    )]
    verbose: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[cfg(feature = "cli")]
    Check {
        #[arg(default_value = None)]
        image: Option<String>,
        #[arg(short, long, default_value_t = false, help = "Enable icons")]
        icons: bool,
        #[arg(
            short,
            long,
            default_value_t = false,
            help = "Output JSON instead of formatted text"
        )]
        raw: bool,
    },
    #[cfg(feature = "server")]
    Serve {
        #[arg(
            short,
            long,
            default_value_t = 8000,
            help = "Use a different port for the server"
        )]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let cfg_path = match cli.config_path.as_str() {
        "" => None,
        path => Some(PathBuf::from(path)),
    };
    if cli.verbose {
        debug!("CLI options:");
        debug!("Config path: {:?}", cfg_path);
        debug!("Socket: {:?}", &cli.socket)
    }
    let cli_config = CliConfig {
        socket: cli.socket,
        verbose: cli.verbose,
        config: load_config(cfg_path),
    };
    if cli.verbose {
        debug!("Config: {}", cli_config.config)
    }
    match &cli.command {
        #[cfg(feature = "cli")]
        Some(Commands::Check { image, icons, raw }) => match image {
            Some(name) => {
                // This is a single update, but it's in a Vec so let's just call it updates
                let updates = get_updates(
                    &[get_image_from_docker_daemon(&cli_config.socket, name).await],
                    &cli_config,
                )
                .await;
                match raw {
                    false => print_updates(&updates, icons),
                    true => print_raw_updates(&updates),
                };
            }
            None => {
                let start = Local::now().timestamp_millis();
                let images = get_images_from_docker_daemon(&cli_config).await;
                match raw {
                    true => {
                        let updates = get_updates(&images, &cli_config).await;
                        print_raw_updates(&updates);
                    }
                    false => {
                        let spinner = Spinner::new();
                        let updates = get_updates(&images, &cli_config).await;
                        spinner.succeed();
                        let end = Local::now().timestamp_millis();
                        print_updates(&updates, icons);
                        info!("✨ Checked {} images in {}ms", updates.len(), end - start);
                    }
                };
            }
        },
        #[cfg(feature = "server")]
        Some(Commands::Serve { port }) => {
            let _ = serve(port, &cli_config).await;
        }
        None => (),
    }
}
