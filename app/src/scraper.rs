use std::{fmt::Debug, future::Future, sync::Arc};

use anyhow::{Error, Result};
use futures::TryFutureExt;
use reqwest::{Client, Response};
use scraper::Html;
use serde::Serialize;
use tokio::task::JoinSet;

use crate::{
    college::{College, GraduateStudent},
    error::Status,
    html::{scrape_html, ScrapperSelectors},
    parser::HtmlRowParser,
};

pub trait PagedRequest: Send {
    fn current_page(&self) -> usize;
    fn set_page(&mut self, page: usize);
}

pub trait PagedResponse: Send {
    fn total_pages(&self) -> Result<usize, Status>;
}

pub trait StudentScraper<Req, Res> {
    fn fetch(&self, req: Req) -> impl Future<Output = Result<Response, Status>> + Send;

    fn deserialize(
        &self,
        response: Response,
    ) -> impl Future<Output = Result<Box<Res>, Status>> + Send;

    fn scrape(
        &self,
        response: Res,
    ) -> impl Future<Output = Result<Vec<Result<GraduateStudent, Status>>, Status>> + Send;
}

impl PagedResponse for String {
    fn total_pages(&self) -> Result<usize, Status> {
        Ok(1)
    }
}

impl PagedRequest for () {
    fn current_page(&self) -> usize {
        0
    }

    fn set_page(&mut self, _page: usize) {}
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScrapperError {
    pub message: String,
}

pub struct SinglePageStudentScrapper {
    pub client: Arc<Client>,
    pub college: College,
    pub selector: ScrapperSelectors,
    pub parser: Box<dyn HtmlRowParser>,
}

impl StudentScraper<(), String> for SinglePageStudentScrapper {
    fn deserialize(
        &self,
        response: Response,
    ) -> impl Future<Output = Result<Box<String>, Status>> + Send {
        response
            .text()
            .map_err(|error| Status::InvalidArgument(Error::from(error)))
            .map_ok(Box::new)
    }

    fn fetch(&self, _: ()) -> impl Future<Output = Result<Response, Status>> + Send {
        self.client
            .get(&self.college.base_url)
            .send()
            .map_err(|error| Status::NotFound(Error::from(error)))
    }

    async fn scrape(
        &self,
        response: String,
    ) -> Result<Vec<Result<GraduateStudent, Status>>, Status> {
        Ok(
            scrape_html(&self.selector, &Html::parse_document(&response))?
                .iter()
                .filter_map(|row| {
                    let Some(student) = self.parser.parse_row(row) else {
                        return None;
                    };

                    Some(Ok(student))
                })
                .collect(),
        )
    }
}

// TODO: Move onto scrapper impl this can then be overriden in liberal arts etc
pub async fn scrape_college<Request, Response>(
    scraper: Arc<impl StudentScraper<Request, Response> + Send + Sync + 'static>,
) -> Result<Vec<Vec<Result<GraduateStudent, Status>>>, Status>
where
    Response: PagedResponse + Debug + Serialize + Send + 'static,
    Request: Serialize + PagedRequest + Debug + Default + Send + 'static,
{
    let initial_request = Request::default();
    let mut current_page = initial_request.current_page();
    let initial_response = *scraper
        .deserialize(scraper.fetch(initial_request).await?)
        .await?;
    let total_pages = initial_response.total_pages()?;
    let mut active_requests = JoinSet::new();
    let mut active_serializations = JoinSet::new();
    let mut active_scrapes = JoinSet::new();
    let mut paged_results = vec![];
    let initial_scraper = scraper.clone();
    current_page += 1;

    active_scrapes.spawn(async move { initial_scraper.scrape(initial_response).await });

    while current_page < total_pages {
        let scraper = scraper.clone();

        active_requests.spawn(async move {
            let mut request = Request::default();
            request.set_page(current_page);
            scraper.fetch(request).await
        });
        current_page += 1;
    }

    while let Some(http_response) = active_requests.join_next().await {
        let scraper = scraper.clone();

        active_serializations.spawn(async move {
            scraper
                .deserialize(http_response.map_err(|error| Status::Internal(Error::from(error)))??)
                .await
        });
    }

    while let Some(list_response) = active_serializations.join_next().await {
        let scraper = scraper.clone();

        active_scrapes.spawn(async move {
            scraper
                .scrape(*list_response.map_err(|error| Status::Internal(Error::from(error)))??)
                .await
        });
    }

    while let Some(result) = active_scrapes.join_next().await {
        let page = result.map_err(|error| Status::Internal(Error::from(error)))??;

        if page.is_empty() {
            continue;
        }

        paged_results.push(page);
    }

    Ok(paged_results)
}
