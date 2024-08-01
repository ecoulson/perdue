use std::sync::Arc;

use anyhow::{Error, Result};
use futures::{prelude::Future, TryFutureExt};
use reqwest::{Client, Response};
use scraper::{ElementRef, Html};

use crate::{
    college::GraduateStudent,
    error::Status,
    html::{scrape_html, ScrapperSelectors},
    parser::HtmlRowParser,
    scraper::StudentScraper,
};

pub struct LiberalArtsScrapper {
    pub client: Arc<Client>,
    pub url: String,
}

struct LiberalArtsParser {}

impl HtmlRowParser for LiberalArtsParser {
    fn is_valid_position(&self, element: &Option<ElementRef<'_>>) -> bool {
        let Some(positions) = self.parse_positions(element) else {
            return false;
        };

        positions.contains(&String::from("Graduate Student"))
    }

    fn parse_positions(&self, element: &Option<ElementRef<'_>>) -> Option<Vec<String>> {
        let Some(element) = element else {
            return None;
        };

        element.text().next().and_then(|position_text| {
            Some(
                position_text
                    .trim()
                    .split(" // ")
                    .map(|part| part.trim().to_string())
                    .collect(),
            )
        })
    }

    fn parse_email(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        element.and_then(|element| Some(element.text().collect::<Vec<&str>>().join("")))
    }
}

impl LiberalArtsScrapper {
    pub fn new(url: &str, client: Arc<Client>) -> Arc<LiberalArtsScrapper> {
        Arc::new(LiberalArtsScrapper {
            url: String::from(url),
            client,
        })
    }
}

impl StudentScraper<(), String> for LiberalArtsScrapper {
    async fn scrape(
        &self,
        response: String,
    ) -> Result<Vec<Result<GraduateStudent, Status>>, Status> {
        let parser = LiberalArtsParser {};

        Ok(scrape_html(
            &ScrapperSelectors {
                directory_row_selector: String::from(".profile-row"),
                position_selector: Some(String::from("td:nth-child(2)")),
                name_selectors: vec![String::from("td:nth-child(1) a")],
                email_selector: Some(String::from("td:nth-child(4)")),
                location_selector: Some(String::from("td:nth-child(5)")),
                department_selector: None,
            },
            &Html::parse_document(&response),
        )?
        .iter()
        .filter_map(|row| {
            let Some(mut student) = parser.parse_row(row) else {
                return None;
            };
            let Some(positions) = parser.parse_positions(&row.position_element) else {
                return None;
            };
            student.department = positions
                .into_iter()
                .find(|position| {
                    position != "Graduate Student"
                        && position != "SIS"
                        && position != "SLC"
                        && position != "Rueff School"
                        && position != "SLC Teaching Assistant"
                        && position != "Teaching Assistant"
                })
                .unwrap_or_else(|| String::new());

            Some(Ok(student))
        })
        .collect())
    }

    fn fetch(&self, _: ()) -> impl Future<Output = Result<Response, Status>> + Send {
        self.client
            .get(&self.url)
            .send()
            .map_err(|error| Status::InvalidArgument(Error::from(error)))
    }

    fn deserialize(
        &self,
        response: Response,
    ) -> impl Future<Output = Result<Box<String>, Status>> + Send {
        response
            .text()
            .map_err(|error| Status::InvalidArgument(Error::from(error)))
            .map_ok(|html| Box::new(html))
    }
}
