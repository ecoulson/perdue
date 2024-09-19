use std::sync::Arc;

use anyhow::{anyhow, Error, Result};
use futures::{prelude::Future, TryFutureExt};
use reqwest::{Client, Response, StatusCode};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::{
    college::{GraduateStudent, Office},
    error::Status,
    html::{scrape_html, ScrapperSelectors},
    parser::{HtmlRowParser, LastNameFirstParser},
    scraper::{PagedRequest, PagedResponse, StudentScraper},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthScrapperRequest {
    action: String,
    query_type: String,
    id: String,
    post_id: usize,
    slug: String,
    canonical_url: String,
    posts_per_page: usize,
    page: usize,
    offset: usize,
    post_type: String,
    repeater: String,
    seo_start_page: usize,
    filters: bool,
    #[serde(rename = "filters_startpage")]
    filters_start_page: usize,
    filters_target: String,
    facets: bool,
    theme_repeater: String,
    meta_key: String,
    meta_value: String,
    meta_compare: String,
    meta_type: String,
    order: String,
    #[serde(rename = "orderby")]
    order_by: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthScrapperResponse {
    html: Option<String>,
    meta: Option<MetaResponse>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetaResponse {
    #[serde(rename = "totalposts")]
    total_posts: usize,
    #[serde(rename = "postcount")]
    post_count: usize,
}

pub struct HealthScrapper {
    pub url: String,
    pub client: Arc<Client>,
}

impl HealthScrapper {
    pub fn new(url: &str, client: Arc<Client>) -> Arc<HealthScrapper> {
        Arc::new(HealthScrapper {
            url: url.to_string(),
            client,
        })
    }
}

impl PagedRequest for HealthScrapperRequest {
    fn set_page(&mut self, page: usize) {
        self.page = page;
    }

    fn current_page(&self) -> usize {
        self.page
    }
}

impl PagedResponse for HealthScrapperResponse {
    fn total_pages(&self) -> Result<usize, Status> {
        match &self.meta {
            Some(response) => Ok(response.total_posts / response.post_count),
            None => Err(Status::NotFound(anyhow!(
                "Metadata not included in response"
            ))),
        }
    }
}

impl StudentScraper<HealthScrapperRequest, HealthScrapperResponse> for HealthScrapper {
    async fn deserialize(&self, response: Response) -> Result<Box<HealthScrapperResponse>, Status> {
        if response.status() != StatusCode::OK {
            return Err(Status::Internal(anyhow!(
                "Failed to make request for page {}",
                response.url()
            )));
        }

        response
            .json()
            .map_err(|error| Status::InvalidArgument(Error::from(error)))
            .await
    }

    fn fetch(
        &self,
        request: HealthScrapperRequest,
    ) -> impl Future<Output = Result<Response, Status>> + Send {
        let query_string = serde_qs::to_string(&request).unwrap();
        self.client
            .get(&format!("{}?{}", self.url, query_string))
            .send()
            .map_err(|error| Status::InvalidArgument(Error::from(error)))
    }

    async fn scrape(
        &self,
        response: HealthScrapperResponse,
    ) -> Result<Vec<Result<GraduateStudent, Status>>, Status> {
        let Some(html) = response.html else {
            return Err(Status::NotFound(anyhow!("HTML not found on response")));
        };
        let table = format!("<table>{}</table>", html);
        let mut student_page_requests = JoinSet::new();
        let mut student_page_serializations = JoinSet::new();
        let mut students = vec![];
        let email_selector = Selector::parse(".email a").unwrap();
        let parser = LastNameFirstParser {};

        scrape_html(
            &ScrapperSelectors {
                directory_row_selector: String::from(".faculty-table--row"),
                name_selectors: vec![String::from(".faculty-table--name a")],
                position_selector: Some(String::from(".faculty-table--title")),
                department_selector: Some(String::from(".faculty-table--department")),
                email_selector: None,
                location_selector: None,
            },
            &Html::parse_fragment(&table),
        )?
        .iter()
        .map(|row| {
            let Some(name_link) = &row.name_elements.first() else {
                return Err(Status::NotFound(anyhow!("Name link element not found")));
            };
            let Some(name_url) = name_link.attr("href") else {
                return Err(Status::NotFound(anyhow!("Name url not found in href")));
            };
            let Some(department_element) = row.department_element else {
                return Err(Status::NotFound(anyhow!("Department element not found")));
            };
            let names = parser.parse_names(&row.name_elements);

            if names.is_empty() {
                return Err(Status::NotFound(anyhow!("No names found")));
            }

            Ok((
                name_url.to_string(),
                GraduateStudent {
                    names,
                    department: department_element
                        .text()
                        .map(|department| department.trim())
                        .collect(),
                    email: String::new(),
                    id: String::new(),
                    office: Office::default(),
                },
            ))
        })
        .for_each(|row_result| match row_result {
            Ok((url, student)) => {
                let client = self.client.clone();
                student_page_requests.spawn(async move { (student, client.get(url).send().await) });
            }
            Err(error) => students.push(Err(error)),
        });

        while let Some(Ok((student, Ok(response)))) = student_page_requests.join_next().await {
            if response.status() != StatusCode::OK {
                let client = self.client.clone();
                let url = response.url().to_string();
                student_page_requests.spawn(async move { (student, client.get(url).send().await) });
                continue;
            }

            student_page_serializations.spawn(async move { (student, response.text().await) });
        }

        while let Some(Ok((mut student, Ok(student_page)))) =
            student_page_serializations.join_next().await
        {
            match Html::parse_document(&student_page)
                .select(&email_selector)
                .next()
            {
                Some(email_element) => {
                    let Some(email) = parser.parse_email(&Some(email_element)) else {
                        students.push(Err(Status::InvalidArgument(anyhow!("Invalid email"))));
                        continue;
                    };
                    let Some(id) = email
                        .trim()
                        .split("@")
                        .next()
                        .and_then(|id| Some(id.to_lowercase()))
                    else {
                        students.push(Err(Status::InvalidArgument(anyhow!("Invalid id in email"))));
                        continue;
                    };
                    student.email = email;
                    student.id = id;
                    students.push(Ok(student));
                }
                None => students.push(Err(Status::NotFound(anyhow!("Email element not found")))),
            };
        }

        Ok(students)
    }
}

impl Default for HealthScrapperRequest {
    fn default() -> Self {
        HealthScrapperRequest {
            action: String::from("alm_get_posts"),
            query_type: String::from("standard"),
            id: String::from("main_directory_listing"),
            post_id: 727,
            slug: String::from("directory"),
            canonical_url: String::from("https%3A%2F%2Fhhs.purdue.edu%2Fabout-hhs%2Fdirectory%2F"),
            posts_per_page: 20,
            page: 0,
            offset: 0,
            post_type: String::from("directory"),
            repeater: String::from("default"),
            seo_start_page: 1,
            filters: true,
            filters_start_page: 0,
            filters_target: String::from("maindirectorylisting"),
            facets: false,
            theme_repeater: String::from("directory-table.php"),
            meta_key: String::from("staff_faculty_type"),
            meta_value: String::from("Graduate Student"),
            meta_type: String::from("CHAR"),
            meta_compare: String::from("IN"),
            order: String::from("DESC"),
            order_by: String::from("date"),
        }
    }
}
