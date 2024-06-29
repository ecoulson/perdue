use std::sync::Arc;

use axum::{routing::get, Router};
use perdue::{
    agriculture::fetch_agriculture_students, college::{list_students, store_students, College}, education::fetch_education_students, salary::{process_salaries, store_salaries}
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// INDIANA DATA SOURCE FROM: https://gateway.ifionline.org/report_builder/Default3a.aspx?rptType=employComp&rpt=EmployComp&rptName=Employee%20Compensation&rpt_unit_in=3186&referrer=byunit#P4072bd793c4545f0aa97626e908ace39_5_oHit0

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "perdue=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let pool_manager = SqliteConnectionManager::file("database/directory");
    let connection_pool = r2d2::Pool::builder()
        .max_size(8)
        .build(pool_manager)
        .unwrap();
    info!("Pipeline Start");
    pipeline(&connection_pool).await;
    info!("Pipeline Done");

    info!("Server is listening.");
    let router = Router::new()
        .route("/", get(list_students))
        .with_state(connection_pool)
        .nest_service(
            "/assets",
            ServeDir::new(format!(
                "{}/assets",
                std::env::current_dir().unwrap().to_str().unwrap()
            )),
        );
    let listener = TcpListener::bind("0.0.0.0:7777").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn pipeline(connection_pool: &Pool<SqliteConnectionManager>) {
    let client = Arc::new(reqwest::Client::new());
    connection_pool
        .get()
        .unwrap()
        .execute(
            "CREATE TABLE IF NOT EXISTS Students (
            Id VARCHAR PRIMARY KEY,
            Name VARCHAR,
            LegalFirstName VARCHAR,
            LegalLastName VARCHAR,
            Email VARCHAR,
            Department VARCHAR,
            Office VARCHAR
            )",
            [],
        )
        .unwrap();
    connection_pool
        .get()
        .unwrap()
        .execute(
            "CREATE INDEX IF NOT EXISTS StudentsByLegalName ON Students (
            LegalFirstName,
            LegalLastName 
            )",
            [],
        )
        .unwrap();
    connection_pool
        .get()
        .unwrap()
        .execute(
            "CREATE TABLE IF NOT EXISTS Salaries (
            StudentId VARCHAR,
            Year INTEGER,
            AmountUsd INTEGER,
            PRIMARY KEY (StudentId, Year)
            )",
            [],
        )
        .unwrap();

    info!("Processing students...");
    info!("Processing college of agriculture...");
    let agriculture_college = College {
        base_url: String::from(
            "https://ag.purdue.edu/api/pi/2021/api/Directory/ListStaffDirectory",
        ),
    };
    let mut paged_students = fetch_agriculture_students(&agriculture_college, &client)
        .await
        .unwrap();
    info!("Done processing college of agriculture...");

    info!("Process college of education...");
    let education_college = College {
        base_url: String::from("https://education.purdue.edu/graduate-directory/")
    };
    paged_students.push(fetch_education_students(&education_college, &client).await);

    info!("Done process college of education...");

    info!("Storing students...");
    for students in paged_students {
        store_students(&students, &connection_pool);
    }
    info!("Done storing students...");
    info!("Done processing students...");


    info!("Processing salaries...");
    let salaries = process_salaries(connection_pool);
    store_salaries(&salaries, connection_pool);
    info!("Done processing salaries...");
}
