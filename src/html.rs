use scraper::{ElementRef, Html, Selector};

use crate::college::Office;

#[derive(Default)]
pub enum NameOrder {
    #[default]
    FirstLast,
    LastFirst,
}

pub struct ScrapperSelectors {
    pub directory_row_selector: String,
    pub name_selector: Option<Vec<String>>,
    pub position_selector: Option<String>,
    pub department_selector: Option<String>,
    pub email_selector: Option<String>,
    pub location_selector: Option<String>,
}

#[derive(Debug, Default)]
pub struct DirectoryRow<'a> {
    pub name_elements: Option<Vec<ElementRef<'a>>>,
    pub position_element: Option<ElementRef<'a>>,
    pub department_element: Option<ElementRef<'a>>,
    pub email_element: Option<ElementRef<'a>>,
    pub location_element: Option<ElementRef<'a>>,
}

// TODO: Refactor to use references when returning a map iterator vs constructing a vec
pub fn scrape_html<'a>(selectors: &'a ScrapperSelectors, dom: &'a Html) -> Vec<DirectoryRow<'a>> {
    let directory_row_selector = Selector::parse(&selectors.directory_row_selector).unwrap();
    let name_selectors = selectors.name_selector.as_ref().and_then(|selectors| {
        Some(
            selectors
                .iter()
                .map(|selector| Selector::parse(&selector).ok().unwrap())
                .collect::<Vec<Selector>>(),
        )
    });
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
    dom.select(&directory_row_selector)
        .map(|entry| DirectoryRow {
            position_element: position_selector
                .as_ref()
                .and_then(|selector| entry.select(&selector).next()),
            name_elements: name_selectors.as_ref().and_then(|selectors| {
                selectors
                    .iter()
                    .map(|selector| entry.select(&selector).next())
                    .collect()
            }),
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
        .collect()
}

pub fn parse_email(element: &ElementRef<'_>) -> Option<String> {
    if let Some(href) = element.attr("href") {
        if !href.contains("@") && href != "#" {
            return None;
        }

        if href == "#" {
            return Some(format!("{}@perdue.edu", element.text().next().unwrap()));
        }

        return Some(href.replace("mailto:", "").trim().to_lowercase());
    }

    element
        .text()
        .next()
        .and_then(|text| Some(text.trim().to_lowercase()))
}

pub fn parse_office(location_element: &ElementRef<'_>) -> Option<Office> {
    let mut location_text = location_element.text();
    let mut location_text_node = location_text.next();

    if location_text_node.is_none() {
        return None;
    }

    if location_text_node.unwrap().trim() == "Office:" {
        location_text_node = location_text.next();
    }

    if location_text_node.is_none() {
        return None;
    }

    let clean_location = location_text_node
        .clone()
        .unwrap()
        .replace("(Lab) ", "")
        .replace("(lab) ", "");
    let mut location = clean_location.trim().split(" ");

    Some(Office {
        building: location.next().unwrap_or_else(|| "").to_string(),
        room: location.next().unwrap_or_else(|| "").to_string(),
    })
}

pub fn parse_names(
    name_elements: &Vec<ElementRef<'_>>,
    order: &NameOrder,
    delimiter: &str,
) -> Vec<String> {
    let name_iterator = name_elements.iter().map(|name_element| {
        name_element
            .text()
            .next()
            .unwrap()
            .trim()
            .replace(".", "")
            .replace("(", "")
            .replace(")", "")
            .split(delimiter)
            .map(String::from)
            .collect::<Vec<String>>()
    });

    if name_elements.len() == 1 {
        return match order {
            NameOrder::LastFirst => name_iterator.flatten().rev().collect(),
            NameOrder::FirstLast => name_iterator.flatten().collect(),
        };
    }

    match order {
        NameOrder::FirstLast => name_iterator.flatten().collect(),
        NameOrder::LastFirst => name_iterator.rev().flatten().collect(),
    }
}

pub fn parse_positions(position_element: &ElementRef<'_>) -> Vec<String> {
    let position_text = position_element.text().next();

    if position_text.is_none() {
        return vec![];
    }

    position_text
        .unwrap()
        .trim()
        .to_string()
        .split(" // ")
        .map(|position| position.trim().to_lowercase())
        .collect()
}

pub fn extract_id_from_email(email: &str) -> String {
    email.split("@").next().unwrap().trim().to_lowercase()
}

pub fn parse_department(department_element: &ElementRef<'_>) -> Option<String> {
    let department_text = department_element.text().next();

    if department_text.is_none() {
        return None;
    }

    Some(department_text.unwrap().trim().replace(",", "").to_string())
}
