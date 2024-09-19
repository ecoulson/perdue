use std::sync::Arc;

use anyhow::{anyhow, Error, Result};
use futures::TryFutureExt;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    college::{GraduateStudent, Office},
    error::Status,
    scraper::{PagedRequest, PagedResponse, StudentScraper},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListAgricultureStaffDirectoryRequest {
    #[serde(rename = "CurrentPageNumber")]
    current_page_number: usize,
    #[serde(rename = "PageSize")]
    page_size: usize,
    #[serde(rename = "OrganizationFilter")]
    organization_filter: Vec<String>,
    #[serde(rename = "ClassificationFilter")]
    classification_filter: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListAgricultureStaffDirectoryResponse {
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
    id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DepartmentResponse {
    #[serde(rename = "department")]
    department: Option<String>,
}

#[derive(Debug)]
pub struct AgricultureScraper {
    pub http_client: Arc<Client>,
    pub base_url: String,
}

impl Default for ListAgricultureStaffDirectoryRequest {
    fn default() -> Self {
        ListAgricultureStaffDirectoryRequest {
            current_page_number: 1,
            page_size: 50,
            organization_filter: vec![String::from("CoA")],
            classification_filter: vec![6],
        }
    }
}

impl PagedRequest for ListAgricultureStaffDirectoryRequest {
    fn set_page(&mut self, page: usize) {
        self.current_page_number = page;
    }

    fn current_page(&self) -> usize {
        self.current_page_number - 1
    }
}

impl PagedResponse for ListAgricultureStaffDirectoryResponse {
    fn total_pages(&self) -> Result<usize, Status> {
        match self.total_pages {
            Some(size) => Ok(size.into()),
            None => Err(Status::NotFound(anyhow!(
                "No total pages found on response",
            ))),
        }
    }
}

impl StudentScraper<ListAgricultureStaffDirectoryRequest, ListAgricultureStaffDirectoryResponse>
    for AgricultureScraper
{
    async fn deserialize(
        &self,
        response: Response,
    ) -> Result<Box<ListAgricultureStaffDirectoryResponse>, Status> {
        if response.status() != StatusCode::OK {
            return Err(Status::Internal(anyhow!(response.status())));
        }

        response
            .json()
            .map_err(|error| Status::InvalidArgument(Error::from(error)))
            .await
    }

    async fn fetch(
        &self,
        request: ListAgricultureStaffDirectoryRequest,
    ) -> Result<Response, Status> {
        self.http_client
            .post(&self.base_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(serde_qs::to_string(&request).unwrap())
            .send()
            .map_err(|error| Status::NotFound(Error::from(error)))
            .await
    }

    async fn scrape(
        &self,
        response: ListAgricultureStaffDirectoryResponse,
    ) -> Result<Vec<Result<GraduateStudent, Status>>, Status> {
        let Some(students) = response.students else {
            return Err(Status::NotFound(anyhow!("No students were found")));
        };

        Ok(students
            .into_iter()
            .filter_map(|student| {
                let mut department = String::new();
                let mut names = vec![];

                if student.id.is_none() && student.email.is_none() {
                    return Some(Err(Status::NotFound(anyhow!("No id or email was found"))));
                }

                let id = match student.id {
                    None => student
                        .email
                        .as_ref()
                        .unwrap()
                        .split("@")
                        .next()
                        .unwrap()
                        .to_lowercase(),
                    Some(id) => id,
                };

                if let Some(first_name) = student.first_name {
                    names.append(&mut first_name.split(" ").map(String::from).collect::<Vec<_>>());
                }

                if let Some(middle_name) = &student.middle_name {
                    names.append(&mut middle_name.split(" ").map(String::from).collect::<Vec<_>>());
                }

                if let Some(last_name) = student.last_name {
                    names.append(&mut last_name.split(" ").map(String::from).collect::<Vec<_>>());
                }

                if let Some(departments) = student.departments {
                    if let Some(first_department) = departments.get(0) {
                        department = first_department.department.clone().unwrap();
                    }
                }

                Some(Ok(GraduateStudent {
                    id,
                    names,
                    email: student.email.unwrap_or(String::new()),
                    department,
                    office: Office {
                        room: student.room.unwrap_or(String::new()),
                        building: student.building.unwrap_or(String::new()),
                    },
                }))
            })
            .collect())
    }
}
