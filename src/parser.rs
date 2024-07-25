use scraper::ElementRef;

use crate::{
    college::{GraduateStudent, Office},
    html::DirectoryRow,
};

pub trait HtmlRowParser: Send + Sync {
    fn is_valid_position(&self, _element: &Option<ElementRef<'_>>) -> bool {
        return true;
    }

    fn parse_names(&self, elements: &Option<Vec<ElementRef<'_>>>) -> Vec<String> {
        let Some(elements) = elements else {
            return vec![];
        };

        elements
            .iter()
            .map(|name_element| {
                name_element
                    .text()
                    .next()
                    .unwrap()
                    .trim()
                    .split(" ")
                    .map(String::from)
                    .collect::<Vec<String>>()
            })
            .flatten()
            .collect()
    }

    fn parse_email(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        let Some(element) = element else {
            return None;
        };

        let Some(href) = element.attr("href") else {
            return None;
        };

        if !href.contains("@") && href != "#" {
            return None;
        }

        return Some(href.replace("mailto:", "").trim().to_lowercase());
    }

    fn parse_office(&self, element: &Option<ElementRef<'_>>) -> Option<Office> {
        let Some(element) = element else {
            return None;
        };
        let mut location_text = element.text();
        let Some(location_text_node) = location_text.next() else {
            return None;
        };
        let mut location = location_text_node.trim().split(" ");

        Some(Office {
            building: location.next().unwrap_or_else(|| "").to_string(),
            room: location.next().unwrap_or_else(|| "").to_string(),
        })
    }

    fn parse_department(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        let Some(element) = element else {
            return None;
        };

        element
            .text()
            .next()
            .and_then(|department_text| Some(department_text.trim().to_string()))
    }

    fn parse_id(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        self.parse_email(element)
            .and_then(|email| Some(email.split("@").next().unwrap().trim().to_lowercase()))
    }

    fn parse_positions(&self, _element: &Option<ElementRef<'_>>) -> Option<Vec<String>> {
        None
    }

    fn parse_row(&self, row: &DirectoryRow<'_>) -> Option<GraduateStudent> {
        if !self.is_valid_position(&row.position_element) {
            return None;
        }

        let mut student = GraduateStudent::default();

        student.names = self.parse_names(&row.name_elements);

        if let Some(office) = self.parse_office(&row.location_element) {
            student.office = office;
        }

        if let Some(email) = self.parse_email(&row.email_element) {
            student.email = email;
        }

        if let Some(id) = self.parse_id(&row.email_element) {
            student.id = id;
        } else {
            return None;
        }

        if let Some(department) = self.parse_department(&row.department_element) {
            student.department = department;
        }

        Some(student)
    }
}

pub struct DefaultRowParser;

pub struct LastNameFirstParser;

pub struct PharmacyParser;

pub struct ChemicalSciencesParser;

pub struct PhysicsAndAstronomyParser;

pub struct VeterinaryMedicineParser;

impl HtmlRowParser for DefaultRowParser {}

impl HtmlRowParser for PharmacyParser {
    fn parse_names(&self, elements: &Option<Vec<ElementRef<'_>>>) -> Vec<String> {
        let Some(elements) = elements else {
            return vec![];
        };

        if elements.len() != 1 {
            return vec![];
        }

        elements[0]
            .text()
            .next()
            .unwrap()
            .trim()
            .replace("(", "")
            .replace(")", "")
            .split(" ")
            .map(String::from)
            .collect::<Vec<String>>()
    }
}

impl HtmlRowParser for LastNameFirstParser {
    fn parse_names(&self, elements: &Option<Vec<ElementRef<'_>>>) -> Vec<String> {
        let Some(elements) = elements else {
            return vec![];
        };

        if elements.len() != 1 {
            return vec![];
        }

        elements[0]
            .text()
            .next()
            .unwrap()
            .trim()
            .split(", ")
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|part| part.split(" "))
            .flatten()
            .map(|part| part.to_string())
            .collect()
    }
}

impl HtmlRowParser for ChemicalSciencesParser {
    fn parse_names(&self, elements: &Option<Vec<ElementRef<'_>>>) -> Vec<String> {
        let Some(elements) = elements else {
            return vec![];
        };

        if elements.len() != 1 {
            return vec![];
        }

        elements[0]
            .text()
            .next()
            .unwrap()
            .trim()
            .replace("(", "")
            .replace(")", "")
            .split(", ")
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|part| part.split(" "))
            .flatten()
            .map(|part| part.to_string())
            .collect()
    }
}

impl HtmlRowParser for PhysicsAndAstronomyParser {
    fn is_valid_position(&self, element: &Option<ElementRef<'_>>) -> bool {
        let Some(element) = element else {
            return false;
        };

        let Some(text) = element.text().next() else {
            return false;
        };

        text.to_lowercase() == "graduate students"
    }

    fn parse_names(&self, elements: &Option<Vec<ElementRef<'_>>>) -> Vec<String> {
        let Some(elements) = elements else {
            return vec![];
        };

        if elements.len() != 1 {
            return vec![];
        }

        elements[0]
            .text()
            .next()
            .unwrap()
            .trim()
            .split(", ")
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|part| part.split(" "))
            .flatten()
            .map(|part| part.to_string())
            .collect()
    }
}

impl HtmlRowParser for VeterinaryMedicineParser {
    fn parse_names(&self, elements: &Option<Vec<ElementRef<'_>>>) -> Vec<String> {
        let Some(elements) = elements else {
            return vec![];
        };

        if elements.len() != 1 {
            return vec![];
        }

        elements[0]
            .text()
            .next()
            .unwrap()
            .trim()
            .replace(".", "")
            .split(", ")
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|part| part.split(" "))
            .flatten()
            .map(|part| part.to_string())
            .collect()
    }
}
