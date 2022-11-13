use core::fmt;
use std::env;

use super::ConfigError;

#[derive(Debug)]
pub enum ConfigSource {
    Os,
    Dotenv,
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ConfigSource::Os => writeln!(f, "Operating System"),
            ConfigSource::Dotenv => writeln!(f, "Dotenv"),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub source: ConfigSource,
    pub port: u16,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let err = match Config::with_dotenv() {
            Ok(cfg) => return Ok(cfg),
            Err(e) => e,
        };

        match err {
            ConfigError::ConfigNotFound(_) => Config::with_os_env(),
            _ => Err(err),
        }
    }

    pub fn with_os_env() -> Result<Self, ConfigError> {
        Ok(Config {
            source: ConfigSource::Os,
            port: env::var("PORT")?.parse()?,
        })
    }

    pub fn with_dotenv() -> Result<Self, ConfigError> {
        Ok(Config {
            source: ConfigSource::Dotenv,
            port: dotenvy::var("PORT")?.parse()?,
        })
    }
}
