use std::{env, fmt, num};

use super::ConfigSource;

#[derive(Debug)]
pub enum ConfigError {
    Os(env::VarError),
    Dotenv(dotenvy::Error),
    BadConfiguration(num::ParseIntError),
    ConfigNotFound(ConfigSource),
}

impl From<env::VarError> for ConfigError {
    fn from(err: env::VarError) -> Self {
        match err {
            env::VarError::NotPresent => ConfigError::ConfigNotFound(ConfigSource::Os),
            _ => ConfigError::Os(err),
        }
    }
}

impl From<dotenvy::Error> for ConfigError {
    fn from(err: dotenvy::Error) -> Self {
        match err {
            dotenvy::Error::EnvVar(env_var_err) => match env_var_err {
                env::VarError::NotPresent => ConfigError::ConfigNotFound(ConfigSource::Dotenv),
                _ => ConfigError::Dotenv(dotenvy::Error::EnvVar(env_var_err)),
            },
            _ => ConfigError::Dotenv(err),
        }
    }
}

impl From<num::ParseIntError> for ConfigError {
    fn from(err: num::ParseIntError) -> Self {
        ConfigError::BadConfiguration(err)
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ConfigError::Os(ref err) => err.fmt(f),
            ConfigError::Dotenv(ref err) => err.fmt(f),
            ConfigError::BadConfiguration(ref err) => err.fmt(f),
            ConfigError::ConfigNotFound(ref src) => {
                writeln!(f, "Unable to find configuration from {}", src)
            }
        }
    }
}
