use std::sync::Arc;

use anyhow::{Error, Result};
use futures::{prelude::Future, TryFutureExt};
use reqwest::{Client, Response};
use scraper::Html;

use crate::{
    college::{GraduateStudent, Office},
    html::{
        extract_id_from_email, parse_email, parse_names, parse_office, parse_positions,
        scrape_html, NameOrder, ScrapperSelectors,
    },
    scrapper::StudentScrapper,
};

pub struct LiberalArtsScrapper {
    pub client: Arc<Client>,
    pub url: String,
}

impl StudentScrapper<(), String> for LiberalArtsScrapper {
    async fn scrape_students(&self, response: String) -> Vec<GraduateStudent> {
        scrape_html(
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
            let names = parse_names(
                row.name_elements.as_ref().unwrap(),
                &NameOrder::FirstLast,
                " ",
            );
            let email = parse_email(&row.email_element.unwrap()).unwrap();
            let position = parse_positions(&row.position_element.unwrap());

            if !position.contains(&String::from("Graduate Student")) {
                return None;
            }

            Some(GraduateStudent {
                department: position
                    .into_iter()
                    .find(|position| {
                        position != "Graduate Student"
                            && position != "SIS"
                            && position != "SLC"
                            && position != "Rueff School"
                            && position != "SLC Teaching Assistant"
                            && position != "Teaching Assistant"
                    })
                    .unwrap_or_else(|| String::new()),
                id: extract_id_from_email(&email),
                email,
                names,
                office: parse_office(&row.location_element.unwrap())
                    .unwrap_or_else(|| Office::default()),
            })
        })
        .collect()
    }

    fn scrape(&self, _: ()) -> impl Future<Output = Result<Response>> + Send {
        self.client.get(&self.url).send().map_err(Error::from)
    }

    fn parse(&self, response: Response) -> impl Future<Output = Result<Box<String>>> + Send {
        response
            .text()
            .map_err(Error::from)
            .map_ok(|html| Box::new(html))
    }
}
