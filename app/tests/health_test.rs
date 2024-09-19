use std::{str::FromStr, sync::Arc};

use mock_http::TestServer;
use perdue::{
    college::{GraduateStudent, Office},
    error::Status,
    health::HealthScrapper,
    scraper::scrape_college,
};
use pretty_assertions::assert_eq;
use reqwest::Client;
use tiny_http::{Header, Response};

async fn invoke_scrape_college(scraper: Arc<HealthScrapper>) -> Vec<Vec<GraduateStudent>> {
    scrape_college(scraper)
        .await
        .expect("Should parse students")
        .into_iter()
        .map(|x| x.into_iter().map(|y| y.unwrap()).collect())
        .collect()
}

#[tokio::test]
async fn fetch_health_students() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        format!(r#"{{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a href=\"{}\">Last, First</a></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {{
                "totalposts": 1,
                "postcount": 1
            }}
        }}"#, server.url()),
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));
    server.add_response(Response::from_string(
        r#"<html><body><div class="email"><a href="mailto:test@purdue.edu">email</a></div></body></html>"#,
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap()));
    let expected_students = vec![vec![GraduateStudent {
        id: String::from("test"),
        names: vec![String::from("First"), String::from("Last")],
        email: String::from("test@purdue.edu"),
        department: String::from("School of Health Sciences"),
        office: Office::default(),
    }]];

    let students =
        invoke_scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert_eq!(students, expected_students)
}

#[tokio::test]
async fn fetch_health_students_failed_fetch() {
    let server = TestServer::new();
    server.add_response(Response::from_string("").with_status_code(500));

    let students =
        scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert!(matches!(students, Err(Status::Internal(_))))
}

#[tokio::test]
async fn fetch_health_students_invalid_json() {
    let server = TestServer::new();
    server.add_response(Response::from_data(vec![]));

    let students =
        scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert!(matches!(students, Err(Status::InvalidArgument(_))))
}

#[tokio::test]
async fn fetch_health_students_no_html() {
    let server = TestServer::new();
    server.add_response(
        Response::from_string(
            r#"{
            "meta": {
                "totalposts": 1,
                "postcount": 1
            }
        }"#,
        )
        .with_header(Header::from_str("Content-Type: application/json").unwrap()),
    );
    server.add_response(Response::from_string(
        r#"<html><body><div class="email"><a href="mailto:test@purdue.edu">email</a></div></body></html>"#,
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap()));

    let error = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert!(matches!(error, Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_invalid_html() {
    let server = TestServer::new();
    server.add_response(
        Response::from_string(
            r#"{
                "html": "awefawefawefawef",
                "meta": {
                    "totalposts": 1,
                    "postcount": 1
                }
            }"#,
        )
        .with_header(Header::from_str("Content-Type: application/json").unwrap()),
    );
    let expected_students: Vec<Vec<GraduateStudent>> = vec![];

    let students =
        invoke_scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert_eq!(students, expected_students)
}

#[tokio::test]
async fn fetch_health_students_no_meta() {
    let server = TestServer::new();
    server.add_response(
        Response::from_string(
            r#"{
                "html": "<div/>"
            }"#,
        )
        .with_header(Header::from_str("Content-Type: application/json").unwrap()),
    );

    let error = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert!(matches!(error, Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_no_name() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        r#"{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {
                "totalposts": 1,
                "postcount": 1
            }
        }"#
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));

    let students = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new())))
        .await
        .unwrap();

    assert!(matches!(students[0][0], Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_no_name_text() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        r#"{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a></a></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {
                "totalposts": 1,
                "postcount": 1
            }
        }"#
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));

    let students = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new())))
        .await
        .unwrap();

    assert!(matches!(students[0][0], Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_no_name_link() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        r#"{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a>Last, First</a></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {
                "totalposts": 1,
                "postcount": 1
            }
        }"#
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));

    let students = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new())))
        .await
        .unwrap();

    assert!(matches!(students[0][0], Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_no_department() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        r#"{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a href=\"http://localhost\">Last, First</a></td></tbody>",
            "meta": {
                "totalposts": 1,
                "postcount": 1
            }
        }"#
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));

    let students = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new())))
        .await
        .unwrap();

    assert!(matches!(students[0][0], Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_retries_when_requesting_student_page() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        format!(r#"{{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a href=\"{}\">Last, First</a></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {{
                "totalposts": 1,
                "postcount": 1
            }}
        }}"#, server.url()),
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));
    server.add_response(Response::from_string("").with_status_code(500));
    server.add_response(Response::from_string(
        r#"<html><body><div class="email"><a href="mailto:test@purdue.edu">email</a></div></body></html>"#,
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap()));
    let expected_students = vec![vec![GraduateStudent {
        id: String::from("test"),
        names: vec![String::from("First"), String::from("Last")],
        email: String::from("test@purdue.edu"),
        department: String::from("School of Health Sciences"),
        office: Office::default(),
    }]];

    let students =
        invoke_scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new()))).await;

    assert_eq!(students, expected_students)
}

#[tokio::test]
async fn fetch_health_students_fails_with_no_email() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        format!(r#"{{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a href=\"{}\">Last, First</a></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {{
                "totalposts": 1,
                "postcount": 1
            }}
        }}"#, server.url()),
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));
    server.add_response(Response::from_string("<html><body></body></html>"));

    let students = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new())))
        .await
        .unwrap();

    assert!(matches!(students[0][0], Err(Status::NotFound(_))))
}

#[tokio::test]
async fn fetch_health_students_no_email() {
    let server = TestServer::new();
    server.add_response(Response::from_string(
        format!(r#"{{
            "html": "<tbody><tr class=\"faculty-table--row\"><td class=\"faculty-table--name\"><a href=\"{}\">Last, First</a></td><td class=\"faculty-table--department\">School of Health Sciences</td></tr></tbody>",
            "meta": {{
                "totalposts": 1,
                "postcount": 1
            }}
        }}"#, server.url()),
    )
    .with_header(Header::from_str("Content-Type: application/json").unwrap()));
    server.add_response(Response::from_string(
        "<html><body><div class=\"email\"><a></a></div></body></html>",
    ));

    let students = scrape_college(HealthScrapper::new(&server.url(), Arc::new(Client::new())))
        .await
        .unwrap();

    assert!(matches!(students[0][0], Err(Status::InvalidArgument(_))))
}
