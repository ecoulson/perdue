use anyhow::anyhow;
use scraper::{ElementRef, Html, Selector};

use crate::error::Status;

pub struct ScrapperSelectors {
    pub directory_row_selector: String,
    pub name_selectors: Vec<String>,
    pub position_selector: Option<String>,
    pub department_selector: Option<String>,
    pub email_selector: Option<String>,
    pub location_selector: Option<String>,
}

#[derive(Debug, Default)]
pub struct DirectoryRow<'a> {
    pub name_elements: Vec<ElementRef<'a>>,
    pub position_element: Option<ElementRef<'a>>,
    pub department_element: Option<ElementRef<'a>>,
    pub email_element: Option<ElementRef<'a>>,
    pub location_element: Option<ElementRef<'a>>,
}

// TODO: Refactor to use references when returning a map iterator vs constructing a vec
pub fn scrape_html<'a>(
    selectors: &'a ScrapperSelectors,
    dom: &'a Html,
) -> Result<Vec<DirectoryRow<'a>>, Status> {
    if !dom.errors.is_empty() {
        return Err(Status::InvalidArgument(anyhow!(dom.errors.join("\n"))));
    }

    let directory_row_selector = Selector::parse(&selectors.directory_row_selector).unwrap();
    let name_selectors = selectors
        .name_selectors
        .iter()
        .map(|selector| Selector::parse(&selector).ok().unwrap())
        .collect::<Vec<Selector>>();
    let position_selector = selectors
        .position_selector
        .as_ref()
        .and_then(|selector| Selector::parse(&selector).ok());
    let department_selector = selectors
        .department_selector
        .as_ref()
        .and_then(|selector| Selector::parse(&selector).ok());
    let email_selector = selectors
        .email_selector
        .as_ref()
        .and_then(|selector| Selector::parse(&selector).ok());
    let location_selector = selectors
        .location_selector
        .as_ref()
        .and_then(|selector| Selector::parse(&selector).ok());

    Ok(dom
        .select(&directory_row_selector)
        .map(|entry| DirectoryRow {
            position_element: position_selector
                .as_ref()
                .and_then(|selector| entry.select(&selector).next()),
            name_elements: name_selectors
                .iter()
                .filter_map(|selector| entry.select(&selector).next())
                .collect(),
            department_element: department_selector
                .as_ref()
                .and_then(|selector| entry.select(&selector).next()),
            email_element: email_selector
                .as_ref()
                .and_then(|selector| entry.select(&selector).next()),
            location_element: location_selector
                .as_ref()
                .and_then(|selector| entry.select(&selector).next()),
        })
        .collect())
}
