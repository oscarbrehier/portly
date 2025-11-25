use clap::{Parser, Subcommand, ValueHint};
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "portly", version, about = "Port discovery utility")]

pub struct Cli {
    #[arg(long, value_parser = validate_port_range)]
    min: Option<u16>,

    #[arg(long, value_parser = validate_port_range)]
    max: Option<u16>,

    #[arg(long)]
    key: Option<String>,

    #[arg(short)]
    app_name: Option<String>,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    forced: bool,

    #[arg(long, default_value = ".portly.env")]
    env_file: PathBuf,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    expand_max: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
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
        #[arg(long, action = clap::ArgAction::SetTrue)]
        expand_max: bool,
    },
}

#[derive(thiserror::Error, Debug)]

pub enum PortError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Port scanning failed")]
    ScanFailed,

    #[error("Invalid range. min ({min}) must be less than max ({max})")]
    InvalidRange { min: u16, max: u16 },
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

    if let Err(e) = fs::write(env_file, content) {
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
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if port_pid.is_empty() || port_pid.parse::<u32>().is_err() {
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

    let pm2_pids: Vec<String> = String::from_utf8_lossy(&output_pm2.stdout)
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.parse::<u32>().is_ok() {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .collect();

    if pm2_pids.is_empty() {
        return false;
    };

    pm2_pids.contains(&port_pid)
}

pub async fn run_port_assignment(
    min: u16,
    max: u16,
    key: String,
    app_name: Option<String>,
    forced: bool,
    expand_max: bool,
    env_file: &PathBuf,
) -> Result<u16, PortError> {
    if min >= max {
        return Err(PortError::InvalidRange { min, max });
    }

    let mut port: Option<u16> = None;

    if !forced {
        if let Some(app) = &app_name {
            if let Some(previous_port) = get_previous_assigned_port(&env_file, &key).await {
                if previous_port >= min && previous_port <= max {
                    let same_process = is_port_owned_by_app(app, previous_port).await;
                    if !same_process {
                        port = Some(previous_port);
                        info!("Reusing previous port: {}", previous_port);
                    }
                }
            }
        }
    }

    if port.is_none() {
        let mut current_max = max;
        const MAX_PORT: u16 = 65535;

        loop {
            match get_available_port(min, current_max).await {
                Ok(found_port) => {
                    info!("Found available port: {}", found_port);
                    port = Some(found_port);
                    break;
                }
                Err(err) => {
                    if !expand_max {
                        error!(
                            "No available port found in range {}-{}: {}",
                            min, current_max, err
                        );
                        break;
                    }

                    let remaining = MAX_PORT.saturating_sub(current_max);

                    if remaining == 0 {
                        error!("Reached max port limit (65535). Cannot expand further");
                        break;
                    }

                    let mut increment = (remaining as f64 * 0.10) as u16;

                    const MIN_INCREMENT: u16 = 50;
                    const MAX_INCREMENT: u16 = 2000;

                    if increment < MIN_INCREMENT {
                        increment = MIN_INCREMENT;
                    }
                    if increment > MAX_INCREMENT {
                        increment = MAX_INCREMENT;
                    }

                    let new_max = current_max.saturating_add(increment).min(MAX_PORT);

                    if new_max > 60000 && current_max <= 60000 {
                        warn!("Approaching high port range (> 60000)");
                    }
                    if new_max > 65000 && current_max <= 65000 {
                        warn!("Very close to port upper limit (> 65000)");
                    }

                    info!(
                        "No port found in {}-{}, expanding max from {} to {} (Inc={})",
                        min, current_max, current_max, new_max, increment
                    );

                    current_max = new_max;
                }
            }
        }
    }

    if let Some(p) = port {
        write_to_env_file(&env_file, &key, p);
        Ok(p)
    } else {
        error!("No port could be assigned");
        Err(PortError::ScanFailed)
    }
}

pub async fn run_default_port_logic(cli: Cli) {
    let min = cli.min.unwrap_or(3000);
    let max = cli.max.unwrap_or(8000);
    let key = cli.key.unwrap_or_else(|| "PORT".into());

    match run_port_assignment(
        min,
        max,
        key,
        cli.app_name,
        cli.forced,
        cli.expand_max,
        &cli.env_file,
    )
    .await
    {
        Ok(port) => println!("{port}"),
        Err(err) => error!("Failed: {err}"),
    }
}

pub async fn run_port_subcommand(
    min: u16,
    max: u16,
    key: String,
    app_name: Option<String>,
    forced: bool,
    expand_max: bool,
    env_file: PathBuf,
) {
    match run_port_assignment(min, max, key, app_name, forced, expand_max, &env_file).await {
        Ok(port) => println!("{port}"),
        Err(err) => error!("Failed: {err}"),
    }
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Port {
            min,
            max,
            key,
            app_name,
            forced,
            env_file,
            expand_max,
        }) => run_port_subcommand(min, max, key, app_name, forced, expand_max, env_file).await,
        None => {
            run_default_port_logic(cli).await;
        }
    }
}
