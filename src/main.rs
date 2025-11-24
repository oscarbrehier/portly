use clap::{Parser, Subcommand};
use regex::Regex;
use std::error::Error;
use std::fmt::format;
use std::fs;
use std::io::BufReader;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Parser)]
#[command(name = "portly", version, about = "")]

pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Port {
        #[arg(long)]
        min: i16,
        #[arg(long)]
        max: i16,
        #[arg(long)]
        key: String,
        #[arg(short)]
        app_name: Option<String>,
    },
}

#[derive(thiserror::Error, Debug)]

pub enum PortError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Port scanning failed")]
    ScanFailed,
}

async fn is_port_available(port: i16) -> bool {
    return tokio::net::TcpListener::bind(("127.0.0.1", port as u16))
        .await
        .is_ok();
}

pub async fn get_available_port(min: i16, max: i16) -> Result<i16, PortError> {
    for port in min..=max {
        if is_port_available(port).await {
            return Ok(port);
        }
    }

    Err(PortError::ScanFailed)
}

pub async fn get_previous_assigned_port(env_name: String) -> Option<i16> {
    let re = Regex::new(&format!("{}=(\\d+)", env_name)).ok()?;
    let env_content: String = fs::read_to_string(".portly.env").unwrap();

    for line in env_content.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(port_str) = caps.get(1) {
                if let Ok(port) = port_str.as_str().parse::<i16>() {
                    if is_port_available(port).await {
                        return Some(port);
                    }
                }
            }
        }
    }

    None
}

fn write_to_env_file(env_name: String, port: i16) {
    let content = format!("export {}={}", env_name, port);

    if fs::write(".portly.env", content).is_err() {
        println!("Failed to write port to .portly.env file");
    };
}

pub async fn is_port_owned_by_app(app_name: String, port: i16) -> bool {
    let output_lsof = Command::new("lsof")
        .args(["-i", &format!(":${}", port.to_string()), "-t"])
        .stdout(Stdio::piped())
        .stdout(Stdio::null())
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Port { min, max, key, app_name } => {
            if let Some(app_name) = app_name {
                let mut port: Option<i16> = None;

                if let Some(previous_assigned_port) =
                    get_previous_assigned_port("PORT".to_string()).await
                {
                    if previous_assigned_port >= min && previous_assigned_port <= max {
                        let is_in_use = is_port_available(previous_assigned_port).await;
                        let is_same_process =
                            is_port_owned_by_app(app_name.to_string(), previous_assigned_port)
                                .await;

                        if !is_in_use && !is_same_process {
                            port = Some(previous_assigned_port);
                            println!(
                                "Previous port is still available. Using: {}",
                                previous_assigned_port
                            );
                        }
                    }
                }

                if !port.is_some() {
                    let available_port = match get_available_port(min, max).await {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("Error finding available port: {}", err);
                            return;
                        }
                    };

                    println!("Found available port: {}", available_port);
                }

                write_to_env_file(key, port.unwrap());

            }

            // get_previous_assigned_port("PORT".to_string());
            // let port = get_available_port(min, max).await.unwrap();
            // write_to_env_file("HELLO".to_string(), port);
            // println!("Found available port: {}", port);
        }
    }
}
