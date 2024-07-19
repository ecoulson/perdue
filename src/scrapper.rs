use std::{fmt::Debug, future::Future, sync::Arc};

use anyhow::{Error, Result};
use futures::TryFutureExt;
use reqwest::{Client, Response};
use scraper::Html;
use serde::Serialize;
use tokio::task::JoinSet;

use crate::{
    college::{College, GraduateStudent},
    html::{
        extract_id_from_email, parse_department, parse_email, parse_names, parse_office,
        parse_positions, scrape_html, NameOrder, ScrapperSelectors,
    },
};

pub trait PagedRequest: Send {
    fn current_page(&self) -> usize;
    fn set_page(&mut self, page: usize);
}

pub trait PagedResponse: Send {
    fn total_pages(&self) -> usize;
}

pub trait StudentScrapper<Req, Res> {
    fn scrape(&self, req: Req) -> impl Future<Output = Result<Response>> + Send;

    fn parse(&self, response: Response) -> impl Future<Output = Result<Box<Res>>> + Send;

    fn scrape_students(&self, response: Res) -> impl Future<Output = Vec<GraduateStudent>> + Send;
}

impl PagedResponse for String {
    fn total_pages(&self) -> usize {
        1
    }
}

impl PagedRequest for () {
    fn current_page(&self) -> usize {
        0
    }

    fn set_page(&mut self, _page: usize) {}
}

pub async fn scrape_college<Request, Response>(
    scraper: Arc<impl StudentScrapper<Request, Response> + Send + Sync + 'static>,
) -> Result<Vec<Vec<GraduateStudent>>, reqwest::Error>
where
    Response: PagedResponse + Debug + Serialize + Send + 'static,
    Request: Serialize + PagedRequest + Debug + Default + Send + 'static,
{
    let initial_request = Request::default();
    let mut current_page = initial_request.current_page();
    let initial_response = *scraper
        .parse(scraper.scrape(initial_request).await.unwrap())
        .await
        .unwrap();
    let total_pages = initial_response.total_pages();
    let mut active_requests = JoinSet::new();
    let mut active_serializations = JoinSet::new();
    let mut active_student_scrapes = JoinSet::new();
    let mut paged_students = vec![];
    let initial_scraper = scraper.clone();

    active_student_scrapes
        .spawn(async move { initial_scraper.scrape_students(initial_response).await });

    while current_page < total_pages {
        let scraper = scraper.clone();

        active_requests.spawn(async move {
            let mut request = Request::default();
            request.set_page(current_page);
            scraper.scrape(request).await
        });
        current_page += 1;
    }

    while let Some(http_response) = active_requests.join_next().await {
        let scraper = scraper.clone();

        active_serializations
            .spawn(async move { scraper.parse(http_response.unwrap().unwrap()).await });
    }

    while let Some(list_response) = active_serializations.join_next().await {
        let scraper = scraper.clone();

        active_student_scrapes.spawn(async move {
            scraper
                .scrape_students(*list_response.unwrap().unwrap())
                .await
        });
    }

    while let Some(student) = active_student_scrapes.join_next().await {
        paged_students.push(student.unwrap());
    }

    Ok(paged_students)
}

pub struct SinglePageStudentScrapper {
    pub client: Arc<Client>,
    pub college: College,
    pub selector: ScrapperSelectors,
    pub order: NameOrder,
    pub allowed_positions: Vec<String>,
    pub delimiter: String,
}

impl StudentScrapper<(), String> for SinglePageStudentScrapper {
    fn parse(&self, response: Response) -> impl Future<Output = Result<Box<String>>> + Send {
        response.text().map_err(Error::from).map_ok(Box::new)
    }

    fn scrape(&self, _: ()) -> impl Future<Output = Result<Response>> + Send {
        self.client
            .get(&self.college.base_url)
            .send()
            .map_err(Error::from)
    }

    async fn scrape_students(&self, response: String) -> Vec<GraduateStudent> {
        scrape_html(&self.selector, &Html::parse_document(&response))
            .iter()
            .filter_map(|row| {
                let mut student = GraduateStudent::default();

                if let Some(name_elements) = &row.name_elements {
                    student.names = parse_names(name_elements, &self.order, &self.delimiter);
                }

                if let Some(email_element) = &row.email_element {
                    let email = parse_email(email_element);

                    if email.is_none() {
                        return None;
                    }

                    student.email = email.unwrap();
                    student.id = extract_id_from_email(&student.email);
                }

                if let Some(location_element) = &row.location_element {
                    student.office = parse_office(location_element)
                        .unwrap_or(self.college.default_office.clone());
                } else {
                    student.office = self.college.default_office.clone();
                }

                if let Some(department_element) = &row.department_element {
                    student.department = parse_department(&department_element)
                        .unwrap_or(self.college.default_department.clone());
                } else {
                    student.department = self.college.default_department.clone();
                }

                if self.allowed_positions.is_empty() {
                    return Some(student);
                }

                if let Some(position_element) = &row.position_element {
                    let positions = parse_positions(position_element);

                    if positions
                        .iter()
                        .find(|position| {
                            self.allowed_positions
                                .iter()
                                .map(|position| position.to_lowercase())
                                .collect::<Vec<_>>()
                                .contains(position)
                        })
                        .is_none()
                    {
                        return None;
                    }
                }

                Some(student)
            })
            .collect()
    }
}
