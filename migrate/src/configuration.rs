use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    pub database: DatabaseConfiguration,
    pub migration_path: String,
}

#[derive(Deserialize)]
pub struct DatabaseConfiguration {
    pub connection_type: DatabaseConnectionType,
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
