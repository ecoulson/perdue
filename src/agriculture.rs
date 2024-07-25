use std::{future::Future, sync::Arc};

use anyhow::{anyhow, Error, Result};
use futures::TryFutureExt;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

use crate::{
    college::{GraduateStudent, Office},
    scrapper::{PagedRequest, PagedResponse, ScrapeResult, ScrapperError, StudentScrapper},
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
pub struct AgricultureScrapper {
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
        self.current_page_number
    }
}

impl PagedResponse for ListAgricultureStaffDirectoryResponse {
    fn total_pages(&self) -> Result<usize> {
        match self.total_pages {
            Some(size) => Ok(size.into()),
            None => Err(anyhow!("Total pages not included in response")),
        }
    }
}

impl StudentScrapper<ListAgricultureStaffDirectoryRequest, ListAgricultureStaffDirectoryResponse>
    for AgricultureScrapper
{
    fn deserialize(
        &self,
        response: Response,
    ) -> impl Future<Output = Result<Box<ListAgricultureStaffDirectoryResponse>>> {
        response.json().map_err(Error::from)
    }

    fn fetch(
        &self,
        request: ListAgricultureStaffDirectoryRequest,
    ) -> impl Future<Output = Result<Response>> {
        self.http_client
            .post(&self.base_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(serde_qs::to_string(&request).unwrap())
            .send()
            .map_err(Error::from)
    }

    async fn scrape(
        &self,
        response: ListAgricultureStaffDirectoryResponse,
    ) -> Result<Vec<ScrapeResult>> {
        let Some(students) = response.students else {
            return Err(anyhow!("No students were found"));
        };

        Ok(students
            .into_iter()
            .map(|student| {
                let mut department = String::new();
                let mut names = vec![];

                if student.id.is_none() && student.email.is_none() {
                    return ScrapeResult::Error(ScrapperError {
                        message: format!("No id found in student {:?}", student),
                    });
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

                ScrapeResult::Success(GraduateStudent {
                    id,
                    names,
                    email: student.email.unwrap_or(String::new()),
                    department,
                    office: Office {
                        room: student.room.unwrap_or(String::new()),
                        building: student.building.unwrap_or(String::new()),
                    },
                })
            })
            .collect())
    }
}
