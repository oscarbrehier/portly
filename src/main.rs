use clap::{Parser, Subcommand, ValueHint};
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "portly", version, about = "")]

pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Port {
        #[arg(long, value_parser = validate_port_range)]
        min: u16,
        #[arg(long, value_parser = validate_port_range)]
        max: u16,
        #[arg(long)]
        key: String,
        #[arg(short)]
        app_name: Option<String>,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        forced: bool,
        #[arg(long, value_hint = ValueHint::FilePath, default_value = ".portly.env")]
        env_file: PathBuf,
    },
}

#[derive(thiserror::Error, Debug)]

pub enum PortError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Port scanning failed")]
    ScanFailed,
}

fn validate_port_range(val: &str) -> Result<u16, String> {
    val.parse::<u16>()
        .map_err(|_| format!("`{val}` is not a valid port number"))
}

async fn is_port_available(port: u16) -> Result<bool, PortError> {
    match tokio::net::TcpListener::bind(("127.0.0.1", port as u16)).await {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                Ok(false)
            } else {
                Err(PortError::Io(e))
            }
        }
    }
}

pub async fn get_available_port(min: u16, max: u16) -> Result<u16, PortError> {
    for port in min..=max {
        if is_port_available(port).await.unwrap_or(false) {
            return Ok(port);
        }
    }

    Err(PortError::ScanFailed)
}

pub async fn get_previous_assigned_port(env_file: &PathBuf, env_name: &str) -> Option<u16> {
    let content: String = fs::read_to_string(env_file).unwrap_or_default();
    let re = Regex::new(&format!("{}=(\\d+)", env_name)).ok()?;

    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(port_str) = caps.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    return match is_port_available(port).await {
                        Ok(true) => Some(port),
                        Ok(false) => None,
                        Err(_) => None,
                    };
                }
            }
        }
    }

    None
}

fn write_to_env_file(env_file: &PathBuf, env_name: &str, port: u16) {
    let content = format!("export {}={}", env_name, port);

    if let Err(e) = fs::write(".portly.env", content) {
        error!("Failed to write port to {}: {}", env_file.display(), e);
    } else {
        info!("Port {} written to {}", port, env_file.display());
    }
}

pub async fn is_port_owned_by_app(app_name: &str, port: u16) -> bool {
    let output_lsof = Command::new("lsof")
        .args(["-i", &format!(":${}", port.to_string()), "-t"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    let Ok(output_lsof) = output_lsof else {
        return false;
    };
    if !output_lsof.status.success() {
        return false;
    };

    let port_pid = String::from_utf8_lossy(&output_lsof.stdout)
        .trim()
        .to_string();

    if port_pid.is_empty() {
        return false;
    };

    let output_pm2 = Command::new("pm2")
        .arg("pid")
        .arg(app_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    let Ok(output_pm2) = output_pm2 else {
        return false;
    };
    if !output_pm2.status.success() {
        return false;
    };

    let pm2_pid = String::from_utf8_lossy(&output_pm2.stdout)
        .trim()
        .to_string();

    if pm2_pid.is_empty() {
        return false;
    };

    port_pid == pm2_pid
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();

    match cli.command {
        Commands::Port {
            min,
            max,
            key,
            app_name,
            forced,
            env_file,
        } => {
            if min >= max {
                error!(
                    "Invalid range. min ({}) must be less than max ({})",
                    min, max
                );
                return;
            };

            let mut port: Option<u16> = None;

            if !forced {
                if let Some(previous_port) = get_previous_assigned_port(&env_file, &key).await {
                    if previous_port >= min && previous_port <= max {
                        let same_process = if let Some(app) = &app_name {
                            is_port_owned_by_app(app, previous_port).await
                        } else {
                            false
                        };

                        if !same_process {
                            port = Some(previous_port);
                            info!("Reusing previous port: {}", previous_port);
                        }
                    }   
                }
            }

            if port.is_none() {
                match get_available_port(min, max).await {
                    Ok(new_port) => {
                        info!("Found available port: {}", new_port);
                        port = Some(new_port)
                    }
                    Err(e) => {
                        error!("Failed to find available port: {}", e);
                        return ;
                    }
                }
            }

            if let Some(p) = port {
                write_to_env_file(&env_file, &key, p);
            } else {
                error!("No port could be assigned");
            }
        }
    }
}
