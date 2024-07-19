use std::sync::Arc;

use anyhow::{Error, Result};
use futures::{prelude::Future, TryFutureExt};
use reqwest::{Client, Response};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::{
    college::{GraduateStudent, Office},
    html::{
        extract_id_from_email, parse_email, parse_names, scrape_html, NameOrder, ScrapperSelectors,
    },
    scrapper::{PagedRequest, PagedResponse, StudentScrapper},
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
    fn total_pages(&self) -> usize {
        if let Some(response) = &self.meta {
            return response.total_posts / response.post_count;
        }

        0
    }
}

impl StudentScrapper<HealthScrapperRequest, HealthScrapperResponse> for HealthScrapper {
    fn parse(
        &self,
        response: Response,
    ) -> impl Future<Output = Result<Box<HealthScrapperResponse>>> + Send {
        response.json().map_err(Error::from)
    }

    fn scrape(
        &self,
        request: HealthScrapperRequest,
    ) -> impl Future<Output = Result<Response>> + Send {
        let query_string = serde_qs::to_string(&request).unwrap();
        self.client
            .get(&format!("{}?{}", self.url, query_string))
            .send()
            .map_err(Error::from)
    }

    async fn scrape_students(&self, response: HealthScrapperResponse) -> Vec<GraduateStudent> {
        let html = format!("<table>{}</table>", &response.html.as_ref().unwrap());
        let mut student_page_requests = JoinSet::new();
        let mut student_page_serializations = JoinSet::new();
        let mut students = vec![];
        let email_selector = Selector::parse(".email a").unwrap();

        scrape_html(
            &ScrapperSelectors {
                directory_row_selector: String::from(".faculty-table--row"),
                name_selector: Some(vec![String::from(".faculty-table--name a")]),
                position_selector: Some(String::from(".faculty-table--title")),
                department_selector: Some(String::from(".faculty-table--department")),
                email_selector: None,
                location_selector: None,
            },
            &Html::parse_document(&html),
        )
        .iter()
        .map(|row| {
            let names = parse_names(
                row.name_elements.as_ref().unwrap(),
                &NameOrder::LastFirst,
                ", ",
            );

            (
                row.name_elements
                    .as_ref()
                    .unwrap()
                    .first()
                    .unwrap()
                    .attr("href")
                    .unwrap()
                    .to_string(),
                GraduateStudent {
                    department: row
                        .department_element
                        .unwrap()
                        .text()
                        .map(|department| department.trim())
                        .collect(),
                    email: String::new(),
                    names,
                    id: String::new(),
                    office: Office::default(),
                },
            )
        })
        .for_each(|(url, student)| {
            let client = self.client.clone();
            student_page_requests.spawn(async move { (student, client.get(url).send().await) });
        });

        while let Some(Ok((student, Ok(response)))) = student_page_requests.join_next().await {
            student_page_serializations.spawn(async move { (student, response.text().await) });
        }

        while let Some(Ok((mut student, Ok(student_page)))) =
            student_page_serializations.join_next().await
        {
            let document = Html::parse_document(&student_page);
            let email_element = document.select(&email_selector).next();

            if let Some(email_element) = email_element {
                student.email = parse_email(&email_element).unwrap();
                student.id = extract_id_from_email(&student.email);
                students.push(student);
            }
        }

        students
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
