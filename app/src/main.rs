use std::sync::Arc;

use configuration::read_configuration;
use perdue::{
    pipeline::start_pipeline,
    server::{start_server, ServerState},
};
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    pub database: DatabaseConfiguration,
    pub migration: DatabaseMigration,
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

/// INDIANA DATA SOURCE FROM: https://gateway.ifionline.org/report_builder/Default3a.aspx?rptType=employComp&rpt=EmployComp&rptName=Employee%20Compensation&rpt_unit_in=3186&referrer=byunit#P4072bd793c4545f0aa97626e908ace39_5_oHit0
#[tokio::main]
async fn main() {
    let configuration = read_configuration("ENVIRONMENT", "CONFIGURATION_PATH")
        .unwrap_or_else(|error| panic!("{}", error.to_string()));
    let pool_manager =
        SqliteConnectionManager::file(configuration.database.connection_type.as_str());
    let connection_pool = r2d2::Pool::builder()
        .max_size(configuration.database.connection_pool.max_size)
        .build(pool_manager)
        .unwrap();
    let state = Arc::new(ServerState {
        connection_pool: connection_pool.clone(),
    });
    let connection = connection_pool.get().unwrap();
    let version: usize = connection
        .query_row("SELECT Version From Migration", [], |row| row.get(0))
        .unwrap_or(0);
    println!("Current migration version: {version}");

    start_pipeline(state.clone());
    start_server(&configuration, state.clone());

    loop {}
}
