// Jackson Coxson

use serde::Deserialize;

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
#[derive(Debug, Deserialize)]
pub struct Config {
    pub fb_username: String,
    pub fb_password: String,
    pub e2ee_pin: Option<String>,
    pub refresh_rate: usize,
    pub gecko: Gecko,
    pub tcp: Tcp,
}

#[derive(Debug, Deserialize)]
pub struct Gecko {
    pub port: u16,
    pub path: String,
    pub headless: bool,
}

#[derive(Debug, Deserialize)]
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
                std::fs::write(path, DEFAULT_CONFIG).expect("Unable to write default config file!");
                toml::from_str(DEFAULT_CONFIG).unwrap()
            }
        }
    }
}
