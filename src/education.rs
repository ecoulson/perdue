use reqwest::Client;
use scraper::{selectable::Selectable, Html, Selector};

use crate::college::{College, GraduateStudent, Office};

pub async fn fetch_education_students(
    college: &College,
    http_client: &Client,
) -> Vec<GraduateStudent> {
    let document = http_client
        .get(&college.base_url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let dom = Html::parse_document(&document);
    let directory_row_selector = Selector::parse(".grad-directory-archive-container").unwrap();
    let name_selector = Selector::parse(".grad-directory-archive-info h2").unwrap();
    let position_selector = Selector::parse(".position").unwrap();
    let department_selector = Selector::parse(".department").unwrap();
    let email_selector = Selector::parse(".grad-directory-archive-contact a").unwrap();

    dom.select(&directory_row_selector)
        .map(|entry| {
            let position = entry
                .select(&position_selector)
                .next()
                .unwrap()
                .text()
                .collect::<Vec<&str>>()
                .join("")
                .to_lowercase();
            let names = entry
                .select(&name_selector)
                .next()
                .unwrap()
                .text().map(|entry| entry.split(" ")).flatten().collect::<Vec<&str>>();
            let department = entry
                .select(&department_selector)
                .next()
                .unwrap()
                .text()
                .collect::<Vec<&str>>()
                .join("");
            let email = entry
                .select(&email_selector)
                .next()
                .unwrap()
                .text()
                .collect::<Vec<&str>>()
                .join("");

            if position != "graduate student" {
                return None;
            }

            Some(GraduateStudent {
                id: email.split("@").next().unwrap().to_string(),
                department: department.split(",").nth(1).unwrap().trim().to_string(),
                email,
                legal_first_name: names[0].to_string(),
                legal_last_name: names[1].to_string(),
                name: names.iter().map(|name| name.to_string()).collect(),
                office: Office {
                    room: String::new(),
                    building: String::new(),
                },
            })
        })
        .filter(|student| student.is_some())
        .map(|student| student.unwrap())
        .collect()
}
