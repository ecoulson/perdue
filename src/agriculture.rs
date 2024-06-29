use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::college::{College, GraduateStudent, Office};

#[derive(Debug, Serialize, Deserialize)]
struct ListAgricultureStaffDirectoryRequest {
    #[serde(rename = "CurrentPageNumber")]
    current_page_number: u16,
    #[serde(rename = "PageSize")]
    page_size: usize,
    #[serde(rename = "OrganizationFilter")]
    organization_filter: Vec<String>,
    #[serde(rename = "ClassificationFilter")]
    classification_filter: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListAgricultureStaffDirectoryResponse {
    #[serde(rename = "Data")]
    students: Option<Vec<AgricultureGraduateStudent>>,
    #[serde(rename = "TotalPages")]
    total_pages: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgricultureGraduateStudent {
    #[serde(rename = "Building")]
    building: Option<String>,
    #[serde(rename = "Email")]
    email: Option<String>,
    #[serde(rename = "FirstName")]
    first_name: Option<String>,
    #[serde(rename = "LastName")]
    last_name: Option<String>,
    #[serde(rename = "MiddleName")]
    middle_name: Option<String>,
    #[serde(rename = "Room")]
    room: Option<String>,
    #[serde(rename = "DepartmentList")]
    departments: Option<Vec<DepartmentResponse>>,
    #[serde(rename = "stralias")]
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DepartmentResponse {
    #[serde(rename = "department")]
    department: Option<String>,
}

pub trait PagedRequest<T> {
    fn increment_page(request: &mut T); 
}

pub async fn fetch_agriculture_students(
    college: &College,
    http_client: &Client,
) -> Result<Vec<Vec<GraduateStudent>>, reqwest::Error> {
    let mut request = ListAgricultureStaffDirectoryRequest {
        current_page_number: 1,
        page_size: 50,
        organization_filter: vec![String::from("CoA")],
        classification_filter: vec![6],
    };
    let initial_response: ListAgricultureStaffDirectoryResponse = http_client
        .post(&college.base_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(serde_qs::to_string(&request).unwrap())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let total_pages = initial_response.total_pages.unwrap();
    let mut active_requests = JoinSet::new();
    let mut active_serializations = JoinSet::new();
    let mut responses = vec![initial_response];

    while request.current_page_number < total_pages {
        request.current_page_number += 1;
        active_requests.spawn({
            http_client
                .post(&college.base_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(serde_qs::to_string(&request).unwrap())
                .send()
        });
    }

    while let Some(http_response) = active_requests.join_next().await {
        active_serializations.spawn({
            http_response
                .unwrap()
                .unwrap()
                .json::<ListAgricultureStaffDirectoryResponse>()
        });
    }

    while let Some(list_response) = active_serializations.join_next().await {
        responses.push(list_response.unwrap().unwrap())
    }

    Ok(responses
        .into_iter()
        .map(|response| {
            response
                .students
                .unwrap()
                .into_iter()
                .map(|student| {
                    let mut department = String::new();
                    let mut name = vec![];

                    if let Some(first_name) = student.first_name {
                        name.push(first_name);
                    }

                    if let Some(middle_name) = student.middle_name {
                        name.push(middle_name);
                    }

                    if let Some(last_name) = student.last_name {
                        name.push(last_name);
                    }

                    if let Some(departments) = student.departments {
                        if let Some(first_department) = departments.get(0) {
                            department = first_department.department.clone().unwrap();
                        }
                    }

                    GraduateStudent {
                        id: student.id,
                        legal_first_name: name[0].clone(),
                        legal_last_name: name[name.len() - 1].clone(),
                        name,
                        email: student.email.unwrap_or(String::new()),
                        department,
                        office: Office {
                            room: student.room.unwrap_or(String::new()),
                            building: student.building.unwrap_or(String::new()),
                        },
                    }
                })
                .collect::<Vec<GraduateStudent>>()
        })
        .collect())
}
