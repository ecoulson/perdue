use std::{env::current_dir, fmt::Display, fs::File, path::PathBuf};

use anyhow::{anyhow, Context, Error};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    pub database: DatabaseConfiguration,
    pub database_migration: DatabaseMigration,
    pub port: u32,
    pub host: String,
}

#[derive(Deserialize)]
pub struct DatabaseMigration {
    pub migration_path: String,
}

#[derive(Deserialize)]
pub struct DatabaseConfiguration {
    pub username: String,
    pub password: String,
    pub database_name: String,
    pub connection_type: DatabaseConnectionType,
    pub connection_pool: DatabaseConnectionPoolConfiguration,
}

#[derive(Deserialize)]
pub struct DatabaseConnectionPoolConfiguration {
    pub max_size: u32,
}

#[derive(Deserialize)]
pub enum DatabaseConnectionType {
    Memory,
    Path(String),
}

impl DatabaseConnectionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Memory => ":memory:",
            Self::Path(path) => path,
        }
    }
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            _ => Err(anyhow!("Couldn't parse environment from '{}'", value)),
        }
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn read_configuration(
    environment_variable: &str,
    configuration_path_variable: &str,
) -> Result<Configuration, Error> {
    let environment = std::env::var(environment_variable)
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .map_err(|error| Error::from(error).context("Failed to parse environment variable"))?;

    match environment {
        Environment::Local => load_env_file().context("Failed to load .env")?,
        _ => (),
    }

    let configuration_path = std::env::var(configuration_path_variable)
        .map(|path| PathBuf::from(path))
        .context("Failed to parse configuration path")?;

    match File::open(&configuration_path) {
        Ok(file) => serde_json::from_reader(file).map_err(|error| {
            Error::from(error).context(format!(
                "Failed to parse configuration file {}",
                configuration_path.to_string_lossy()
            ))
        }),
        Err(error) => Err(Error::from(error).context(format!(
            "Failed to open configuration {}",
            configuration_path.to_string_lossy()
        ))),
    }
}

fn load_env_file() -> Result<(), Error> {
    let current_dir = current_dir()
        .context("Failed to get current directory")?
        .join(".env");
    let file = std::fs::read_to_string(&current_dir).context(format!(
        "Failed to read .env {}",
        current_dir.to_string_lossy()
    ))?;

    for line in file.split("\n") {
        if line.len() == 0 {
            continue;
        }

        let mut parts = line.split("=");
        std::env::set_var(parts.next().unwrap(), parts.next().unwrap());
    }

    Ok(())
}
