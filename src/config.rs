// Jackson Coxson

use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG: &str = r#"# Holly Config
fb_username = "asdfasdf@urmom.com"
fb_password = "monkey123"
refresh_rate = 3000

[gecko]
port = 4444
path = "/home/user/gecko"
headless = true

[tcp]
port = 8011
host = "127.0.0.1"
"#;

/// Holly configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub fb_username: String,
    pub fb_password: String,
    pub e2ee_pin: Option<String>,
    pub refresh_rate: usize,
    pub latency: usize,
    pub gecko: Gecko,
    pub tcp: Tcp,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gecko {
    pub port: u16,
    pub path: String,
    pub headless: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tcp {
    pub port: u16,
    pub host: String,
}

impl Config {
    /// Loads the config file
    pub fn load() -> Self {
        // Determine if HOLLY_CONFIG_PATH is set
        let path = std::env::var("HOLLY_CONFIG_PATH").unwrap_or("config.toml".to_string());

        // Load the file or create it
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).expect("Invalid config file"),
            Err(_) => {
                // Create new configuration file
                if atty::is(atty::Stream::Stdout) {
                    println!("Looks like you don't have a Holly config, let's set one up!");
                    let new_config = Config {
                        fb_username: Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Enter your Facebook bot username")
                            .interact()
                            .unwrap(),
                        fb_password: Password::with_theme(&ColorfulTheme::default())
                            .with_prompt("Enter your Facebook bot password")
                            .interact()
                            .unwrap(),
                        e2ee_pin: loop {
                            let pin: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt(
                                    "Enter your Facebook bot e2ee pin (leave empty for none)",
                                )
                                .allow_empty(true)
                                .interact()
                                .unwrap();
                            if pin.is_empty() {
                                break None;
                            } else if pin.parse::<u32>().is_ok() {
                                break Some(pin);
                            }
                            println!("Enter a number...");
                        },
                        refresh_rate: loop {
                            let rate: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt(
                                    "Choose a refresh rate in miliseconds for message scanning. A higher refresh rate will mean faster message reading, but will increase CPU usage and accuracy.",
                                )
                                .default("3000".to_string())
                                .interact()
                                .unwrap();
                            if let Ok(rate) = rate.parse::<usize>() {
                                break rate;
                            }
                            println!("Enter a number...");
                        },
                        latency: loop {
                            let rate: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt(
                                    "Enter the latency on context changes. Holly will be quicker with a lower latency, but less accurate with poor hardware and network speed.",
                                )
                                .default("1000".to_string())
                                .interact()
                                .unwrap();
                            if let Ok(rate) = rate.parse::<usize>() {
                                break rate;
                            }
                            println!("Enter a number...");
                        },
                        gecko: Gecko {
                            port: loop {
                                let rate: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt(
                                    "Enter the port that geckodriver is listening on. Defaults to 4444.",
                                )
                                .default("4444".to_string())
                                .interact()
                                .unwrap();
                                if let Ok(rate) = rate.parse::<u16>() {
                                    break rate;
                                }
                                println!("Enter a number...");
                            },
                            path: {
                                println!("Get geckodriver at https://github.com/mozilla/geckodriver/releases. Unzip it and place the file where this program can find it.");
                                println!(
                                    "You need {} for {}.",
                                    std::env::consts::ARCH,
                                    std::env::consts::OS
                                );
                                Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Enter the path to geckodriver")
                                    .default("geckodriver".to_string())
                                    .interact()
                                    .unwrap()
                            },
                            headless: Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Headless? (don't show the Firefox window)")
                                .item("Yes")
                                .item("No")
                                .interact()
                                .unwrap()
                                == 0,
                        },
                        tcp: Tcp {
                            port: loop {
                                let port: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt(
                                        "Choose a port to listen for children processes on",
                                    )
                                    .default("8011".to_string())
                                    .interact()
                                    .unwrap();
                                if let Ok(rate) = port.parse::<u16>() {
                                    break rate;
                                }
                                println!("Enter a number...");
                            },
                            host: loop {
                                let ip: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Enter the IP to listen on")
                                    .default("127.0.0.1".to_string())
                                    .interact()
                                    .unwrap();
                                if ip.parse::<std::net::Ipv4Addr>().is_ok() {
                                    break ip;
                                }
                                println!("Enter an IP address...");
                            },
                        },
                    };
                    std::fs::write(path, toml::to_string(&new_config).unwrap())
                        .expect("Unable to write new config file");
                    new_config
                } else {
                    println!("WROTE NEW CONFIG FILE! Edit it at {path}. You probably don't want the default values.");
                    std::fs::write(path, DEFAULT_CONFIG)
                        .expect("Unable to write default config file!");
                    toml::from_str(DEFAULT_CONFIG).unwrap()
                }
            }
        }
    }
}
