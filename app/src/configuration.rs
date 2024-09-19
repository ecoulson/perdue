use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    pub database: DatabaseConfiguration,
    pub files: Files,
    pub port: u32,
    pub host: String,
}

#[derive(Deserialize)]
pub struct Files {
    pub salaries_directory: String,
    pub assets_directory: String,
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
