use std::sync::Arc;

use perdue::{
    configuration::read_configuration,
    pipeline::start_pipeline,
    server::{start_server, ServerState},
};
use r2d2_sqlite::SqliteConnectionManager;

/// INDIANA DATA SOURCE FROM: https://gateway.ifionline.org/report_builder/Default3a.aspx?rptType=employComp&rpt=EmployComp&rptName=Employee%20Compensation&rpt_unit_in=3186&referrer=byunit#P4072bd793c4545f0aa97626e908ace39_5_oHit0
#[tokio::main]
async fn main() {
    let configuration = read_configuration("ENVIRONMENT", "LOCAL_CONFIGURATION")
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

    start_pipeline(state.clone());
    start_server(&configuration, state.clone());

    loop {}
}
