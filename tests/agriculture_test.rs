use std::{collections::LinkedList, io::Cursor, str::FromStr, sync::Arc};

use perdue::{
    agriculture::AgricultureScraper,
    college::{GraduateStudent, Office},
    error::Status,
    scraper::scrape_college,
};
use pretty_assertions::assert_eq;
use reqwest::Client;
use tiny_http::{Header, Response, Server};
use tokio::test;

fn create_test_server(responses: Vec<Response<Cursor<Vec<u8>>>>) -> Arc<Server> {
    let server = Arc::new(Server::http("0.0.0.0:0").unwrap());
    let mut responses = LinkedList::from_iter(responses.into_iter());
    let request_server = server.clone();

    std::thread::spawn(move || {
        while let Ok(request) = request_server.recv() {
            let Some(response) = responses.pop_front() else {
                request
                    .respond(Response::from_string("No responses queued").with_status_code(500))
                    .unwrap();
                return;
            };

            request.respond(response).unwrap();
        }
    });

    server
}

async fn invoke_scrape_college(scraper: AgricultureScraper) -> Vec<Vec<GraduateStudent>> {
    scrape_college(Arc::new(scraper))
        .await
        .expect("Should parse students")
        .into_iter()
        .map(|x| x.into_iter().map(|y| y.unwrap()).collect())
        .collect()
}

#[test]
async fn scrape_agriculture_test_single_page() {
    let response = r#"
        {
            "IsSuccess": true,
            "ReturnMessage": [],
            "TotalPages": 1,
            "TotalRows": 536,
            "PageSize": 50,
            "CurrentPageNumber": 1,
            "SortExpression": null,
            "SortDirection": null,
            "SearchExpression": null,
            "ShowArchived": null,
            "ShowSuppressed": false,
            "DepartmentFilter": null,
            "ClassificationFilter": [
                6
            ],
            "ExtensionFilter": null,
            "OrganizationFilter": [
                "CoA"
            ],
            "Data": [
                {
                    "stralias": "aaarstad",
                    "LastName": "Aarstad",
                    "FirstName": "Anna",
                    "MiddleName": "Kay",
                    "Suffix": "",
                    "Email": "aaarstad@purdue.edu",
                    "Gender": "F",
                    "ProNouns": "",
                    "HireDate": "2023-05-15T00:00:00",
                    "Building": "KRAN",
                    "Room": "",
                    "PhoneAreaCode": "",
                    "Phone": "",
                    "FaxAreaCode": "",
                    "Fax": "",
                    "Title": "Graduate Research Assistant",
                    "Address1": "403 W. State Street",
                    "Address2": null,
                    "City": "West Lafayette",
                    "State": "IN",
                    "Zip": "47907",
                    "officeAddressId": "00000000-0000-0000-0000-000000000000",
                    "DepartmentList": [
                        {
                            "IsPrimary": true,
                            "ClassificationId": 6,
                            "classification": "Graduate Student",
                            "startDate": "1900-01-01T00:00:00",
                            "title": "Graduate Research Assistant",
                            "Id": 10,
                            "organization": "CoA",
                            "parentOrganization": "Academic Departments",
                            "department": "Agricultural Economics",
                            "archived": false
                        }
                    ],
                    "ClassificationList": [
                        {
                            "Id": 6,
                            "organization": "CoA",
                            "classification": "Graduate Student",
                            "archived": false,
                            "IsPrimary": true
                        }
                    ],
                    "ExtensionList": []
                },
                {
                    "stralias": "abdelhas",
                    "LastName": "Abdelhaseib",
                    "FirstName": "Maha",
                    "MiddleName": "Mohamed Usama",
                    "Suffix": null,
                    "Email": "maha@purdue.edu",
                    "Gender": "F",
                    "ProNouns": "",
                    "HireDate": "2023-05-15T00:00:00",
                    "Building": "CRTN",
                    "Room": "2088",
                    "PhoneAreaCode": "",
                    "Phone": "",
                    "FaxAreaCode": "",
                    "Fax": "",
                    "Title": "Graduate Research Assistant",
                    "Address1": "270 S Russell Street",
                    "Address2": "",
                    "City": "West Lafayette",
                    "State": "IN",
                    "Zip": "47907",
                    "officeAddressId": "00000000-0000-0000-0000-000000000000",
                    "DepartmentList": [
                        {
                            "IsPrimary": true,
                            "ClassificationId": 6,
                            "classification": "Graduate Student",
                            "startDate": "1900-01-01T00:00:00",
                            "title": "Graduate Research Assistant",
                            "Id": 12,
                            "organization": "CoA",
                            "parentOrganization": "Academic Departments",
                            "department": "Animal Sciences",
                            "archived": false
                        }
                    ],
                    "ClassificationList": [
                        {
                            "Id": 6,
                            "organization": "CoA",
                            "classification": "Graduate Student",
                            "archived": false,
                            "IsPrimary": true
                        }
                    ],
                    "ExtensionList": []
                }
            ]
        }"#;
    let server = create_test_server(vec![Response::from_string(response)
        .with_header(Header::from_str("Content-Type: application/json").unwrap())]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };
    let expected_results = vec![vec![
        GraduateStudent {
            id: String::from("aaarstad"),
            email: String::from("aaarstad@purdue.edu"),
            department: String::from("Agricultural Economics"),
            names: vec![
                String::from("Anna"),
                String::from("Kay"),
                String::from("Aarstad"),
            ],
            office: Office {
                building: String::from("KRAN"),
                room: String::from(""),
            },
        },
        GraduateStudent {
            id: String::from("abdelhas"),
            email: String::from("maha@purdue.edu"),
            department: String::from("Animal Sciences"),
            names: vec![
                String::from("Maha"),
                String::from("Mohamed"),
                String::from("Usama"),
                String::from("Abdelhaseib"),
            ],
            office: Office {
                building: String::from("CRTN"),
                room: String::from("2088"),
            },
        },
    ]];

    let results = invoke_scrape_college(scraper).await;

    assert_eq!(results, expected_results);
}

#[test]
async fn scrape_agriculture_test_multiple_pages() {
    let page_1 = r#"
    {
        "IsSuccess": true,
        "ReturnMessage": [],
        "TotalPages": 2,
        "TotalRows": 2,
        "PageSize": 1,
        "CurrentPageNumber": 1,
        "SortExpression": null,
        "SortDirection": null,
        "SearchExpression": null,
        "ShowArchived": null,
        "ShowSuppressed": false,
        "DepartmentFilter": null,
        "ClassificationFilter": [
            6
        ],
        "ExtensionFilter": null,
        "OrganizationFilter": [
            "CoA"
        ],
        "Data": [
            {
                "stralias": "aaarstad",
                "LastName": "Aarstad",
                "FirstName": "Anna",
                "MiddleName": "Kay",
                "Suffix": "",
                "Email": "aaarstad@purdue.edu",
                "Gender": "F",
                "ProNouns": "",
                "HireDate": "2023-05-15T00:00:00",
                "Building": "KRAN",
                "Room": "",
                "PhoneAreaCode": "",
                "Phone": "",
                "FaxAreaCode": "",
                "Fax": "",
                "Title": "Graduate Research Assistant",
                "Address1": "403 W. State Street",
                "Address2": null,
                "City": "West Lafayette",
                "State": "IN",
                "Zip": "47907",
                "officeAddressId": "00000000-0000-0000-0000-000000000000",
                "DepartmentList": [
                    {
                        "IsPrimary": true,
                        "ClassificationId": 6,
                        "classification": "Graduate Student",
                        "startDate": "1900-01-01T00:00:00",
                        "title": "Graduate Research Assistant",
                        "Id": 10,
                        "organization": "CoA",
                        "parentOrganization": "Academic Departments",
                        "department": "Agricultural Economics",
                        "archived": false
                    }
                ],
                "ClassificationList": [
                    {
                        "Id": 6,
                        "organization": "CoA",
                        "classification": "Graduate Student",
                        "archived": false,
                        "IsPrimary": true
                    }
                ],
                "ExtensionList": []
            }
        ]
    }"#;
    let page_2 = r#"
    {
        "IsSuccess": true,
        "ReturnMessage": [],
        "TotalPages": 2,
        "TotalRows": 2,
        "PageSize": 1,
        "CurrentPageNumber": 2,
        "SortExpression": null,
        "SortDirection": null,
        "SearchExpression": null,
        "ShowArchived": null,
        "ShowSuppressed": false,
        "DepartmentFilter": null,
        "ClassificationFilter": [
            6
        ],
        "ExtensionFilter": null,
        "OrganizationFilter": [
            "CoA"
        ],
        "Data": [
            {
                "stralias": "abdelhas",
                "LastName": "Abdelhaseib",
                "FirstName": "Maha",
                "MiddleName": "Mohamed Usama",
                "Suffix": null,
                "Email": "maha@purdue.edu",
                "Gender": "F",
                "ProNouns": "",
                "HireDate": "2023-05-15T00:00:00",
                "Building": "CRTN",
                "Room": "2088",
                "PhoneAreaCode": "",
                "Phone": "",
                "FaxAreaCode": "",
                "Fax": "",
                "Title": "Graduate Research Assistant",
                "Address1": "270 S Russell Street",
                "Address2": "",
                "City": "West Lafayette",
                "State": "IN",
                "Zip": "47907",
                "officeAddressId": "00000000-0000-0000-0000-000000000000",
                "DepartmentList": [
                    {
                        "IsPrimary": true,
                        "ClassificationId": 6,
                        "classification": "Graduate Student",
                        "startDate": "1900-01-01T00:00:00",
                        "title": "Graduate Research Assistant",
                        "Id": 12,
                        "organization": "CoA",
                        "parentOrganization": "Academic Departments",
                        "department": "Animal Sciences",
                        "archived": false
                    }
                ],
                "ClassificationList": [
                    {
                        "Id": 6,
                        "organization": "CoA",
                        "classification": "Graduate Student",
                        "archived": false,
                        "IsPrimary": true
                    }
                ],
                "ExtensionList": []
            }
        ]
    }"#;
    let server = create_test_server(vec![
        Response::from_string(page_1)
            .with_header(Header::from_str("Content-Type: application/json").unwrap()),
        Response::from_string(page_2)
            .with_header(Header::from_str("Content-Type: application/json").unwrap()),
    ]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };
    let expected_results = vec![
        vec![GraduateStudent {
            id: String::from("aaarstad"),
            email: String::from("aaarstad@purdue.edu"),
            department: String::from("Agricultural Economics"),
            names: vec![
                String::from("Anna"),
                String::from("Kay"),
                String::from("Aarstad"),
            ],
            office: Office {
                building: String::from("KRAN"),
                room: String::from(""),
            },
        }],
        vec![GraduateStudent {
            id: String::from("abdelhas"),
            email: String::from("maha@purdue.edu"),
            department: String::from("Animal Sciences"),
            names: vec![
                String::from("Maha"),
                String::from("Mohamed"),
                String::from("Usama"),
                String::from("Abdelhaseib"),
            ],
            office: Office {
                building: String::from("CRTN"),
                room: String::from("2088"),
            },
        }],
    ];

    let results = invoke_scrape_college(scraper).await;

    assert_eq!(results, expected_results);
}

#[test]
async fn scrape_agriculture_page_no_id() {
    let page_1 = r#"
    {
        "IsSuccess": true,
        "ReturnMessage": [],
        "TotalPages": 1,
        "TotalRows": 1,
        "PageSize": 1,
        "CurrentPageNumber": 1,
        "SortExpression": null,
        "SortDirection": null,
        "SearchExpression": null,
        "ShowArchived": null,
        "ShowSuppressed": false,
        "DepartmentFilter": null,
        "ClassificationFilter": [
            6
        ],
        "ExtensionFilter": null,
        "OrganizationFilter": [
            "CoA"
        ],
        "Data": [
            {
                "LastName": "Aarstad",
                "FirstName": "Anna",
                "MiddleName": "Kay",
                "Suffix": "",
                "Email": "aaarstad@purdue.edu",
                "Gender": "F",
                "ProNouns": "",
                "HireDate": "2023-05-15T00:00:00",
                "Building": "KRAN",
                "Room": "",
                "PhoneAreaCode": "",
                "Phone": "",
                "FaxAreaCode": "",
                "Fax": "",
                "Title": "Graduate Research Assistant",
                "Address1": "403 W. State Street",
                "Address2": null,
                "City": "West Lafayette",
                "State": "IN",
                "Zip": "47907",
                "officeAddressId": "00000000-0000-0000-0000-000000000000",
                "DepartmentList": [
                    {
                        "IsPrimary": true,
                        "ClassificationId": 6,
                        "classification": "Graduate Student",
                        "startDate": "1900-01-01T00:00:00",
                        "title": "Graduate Research Assistant",
                        "Id": 10,
                        "organization": "CoA",
                        "parentOrganization": "Academic Departments",
                        "department": "Agricultural Economics",
                        "archived": false
                    }
                ],
                "ClassificationList": [
                    {
                        "Id": 6,
                        "organization": "CoA",
                        "classification": "Graduate Student",
                        "archived": false,
                        "IsPrimary": true
                    }
                ],
                "ExtensionList": []
            }
        ]
    }"#;
    let server = create_test_server(vec![Response::from_string(page_1)
        .with_header(Header::from_str("Content-Type: application/json").unwrap())]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };
    let expected_students = vec![vec![GraduateStudent {
        id: String::from("aaarstad"),
        email: String::from("aaarstad@purdue.edu"),
        department: String::from("Agricultural Economics"),
        names: vec![
            String::from("Anna"),
            String::from("Kay"),
            String::from("Aarstad"),
        ],
        office: Office {
            building: String::from("KRAN"),
            room: String::from(""),
        },
    }]];

    let students = invoke_scrape_college(scraper).await;

    assert_eq!(students, expected_students);
}

#[test]
async fn scrape_agriculture_page_no_id_or_email() {
    let page_1 = r#"
    {
        "IsSuccess": true,
        "ReturnMessage": [],
        "TotalPages": 1,
        "TotalRows": 1,
        "PageSize": 1,
        "CurrentPageNumber": 1,
        "SortExpression": null,
        "SortDirection": null,
        "SearchExpression": null,
        "ShowArchived": null,
        "ShowSuppressed": false,
        "DepartmentFilter": null,
        "ClassificationFilter": [
            6
        ],
        "ExtensionFilter": null,
        "OrganizationFilter": [
            "CoA"
        ],
        "Data": [
            {
                "LastName": "Aarstad",
                "FirstName": "Anna",
                "MiddleName": "Kay",
                "Suffix": "",
                "Gender": "F",
                "ProNouns": "",
                "HireDate": "2023-05-15T00:00:00",
                "Building": "KRAN",
                "Room": "",
                "PhoneAreaCode": "",
                "Phone": "",
                "FaxAreaCode": "",
                "Fax": "",
                "Title": "Graduate Research Assistant",
                "Address1": "403 W. State Street",
                "Address2": null,
                "City": "West Lafayette",
                "State": "IN",
                "Zip": "47907",
                "officeAddressId": "00000000-0000-0000-0000-000000000000",
                "DepartmentList": [
                    {
                        "IsPrimary": true,
                        "ClassificationId": 6,
                        "classification": "Graduate Student",
                        "startDate": "1900-01-01T00:00:00",
                        "title": "Graduate Research Assistant",
                        "Id": 10,
                        "organization": "CoA",
                        "parentOrganization": "Academic Departments",
                        "department": "Agricultural Economics",
                        "archived": false
                    }
                ],
                "ClassificationList": [
                    {
                        "Id": 6,
                        "organization": "CoA",
                        "classification": "Graduate Student",
                        "archived": false,
                        "IsPrimary": true
                    }
                ],
                "ExtensionList": []
            }
        ]
    }"#;
    let server = create_test_server(vec![Response::from_string(page_1)
        .with_header(Header::from_str("Content-Type: application/json").unwrap())]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };

    let students = scrape_college(Arc::new(scraper))
        .await
        .expect("Should fail due to empty body");

    assert!(matches!(students[0][0], Err(_)));
}

#[test]
async fn scrape_agriculture_page_no_students() {
    let page_1 = r#"
    {
        "IsSuccess": true,
        "ReturnMessage": [],
        "TotalPages": 1,
        "TotalRows": 1,
        "PageSize": 1,
        "CurrentPageNumber": 1,
        "SortExpression": null,
        "SortDirection": null,
        "SearchExpression": null,
        "ShowArchived": null,
        "ShowSuppressed": false,
        "DepartmentFilter": null,
        "ClassificationFilter": [
            6
        ],
        "ExtensionFilter": null,
        "OrganizationFilter": [
            "CoA"
        ],
        "Data": []
    }"#;
    let server = create_test_server(vec![Response::from_string(page_1)
        .with_header(Header::from_str("Content-Type: application/json").unwrap())]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };

    let students = invoke_scrape_college(scraper).await;

    assert!(students.is_empty());
}

#[test]
async fn scrape_agriculture_page_empty_json() {
    let page_1 = r#"{}"#;
    let server = create_test_server(vec![Response::from_string(page_1)
        .with_header(Header::from_str("Content-Type: application/json").unwrap())]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };

    let error = scrape_college(Arc::new(scraper)).await;

    assert!(matches!(error, Err(Status::NotFound(_))));
}

#[test]
async fn scrape_agriculture_with_error() {
    let server = create_test_server(vec![Response::from_string("{}")
        .with_status_code(500)
        .with_header(Header::from_str("Content-Type: application/json").unwrap())]);
    let scraper = AgricultureScraper {
        http_client: Arc::new(Client::new()),
        base_url: format!("http://{}", server.server_addr().to_string()),
    };

    let error = scrape_college(Arc::new(scraper)).await;
    dbg!(&error);

    assert!(matches!(error, Err(Status::Internal(_))));
}
