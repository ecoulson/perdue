use std::sync::Arc;

use anyhow::{Error, Result};
use futures::{prelude::Future, TryFutureExt};
use reqwest::{Client, Response};
use scraper::{ElementRef, Html};

use crate::{
    html::{scrape_html, ScrapperSelectors},
    parser::HtmlRowParser,
    scrapper::{ScrapeResult, StudentScrapper},
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

        return positions.contains(&String::from("Graduate Student"));
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
                    .map(String::from)
                    .collect(),
            )
        })
    }
}

impl StudentScrapper<(), String> for LiberalArtsScrapper {
    async fn scrape(&self, response: String) -> Result<Vec<ScrapeResult>> {
        let parser = LiberalArtsParser {};

        Ok(scrape_html(
            &ScrapperSelectors {
                directory_row_selector: String::from(".profile-row"),
                position_selector: Some(String::from("td:nth-child(2)")),
                name_selector: Some(vec![String::from("td:nth-child(1) a")]),
                email_selector: Some(String::from("td:nth-child(4)")),
                location_selector: Some(String::from("td:nth-child(5)")),
                department_selector: None,
            },
            &Html::parse_document(&response),
        )
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

            Some(ScrapeResult::Success(student))
        })
        .collect())
    }

    fn fetch(&self, _: ()) -> impl Future<Output = Result<Response>> + Send {
        self.client.get(&self.url).send().map_err(Error::from)
    }

    fn deserialize(&self, response: Response) -> impl Future<Output = Result<Box<String>>> + Send {
        response
            .text()
            .map_err(Error::from)
            .map_ok(|html| Box::new(html))
    }
}
