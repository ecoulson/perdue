use scraper::ElementRef;

use crate::{
    college::{GraduateStudent, Office},
    html::DirectoryRow,
};

pub trait HtmlRowParser: Send + Sync {
    fn is_valid_position(&self, _element: &Option<ElementRef<'_>>) -> bool {
        true
    }

    fn parse_names(&self, elements: &Vec<ElementRef<'_>>) -> Vec<String> {
        elements
            .iter()
            .filter_map(|name_element| match name_element.text().next() {
                Some(element) => Some(
                    element
                        .trim()
                        .split(" ")
                        .map(String::from)
                        .collect::<Vec<String>>(),
                ),
                None => None,
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

        Some(href.replace("mailto:", "").trim().to_lowercase())
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
        self.parse_email(element).and_then(|email| {
            email
                .trim()
                .split("@")
                .next()
                .and_then(|id| Some(id.to_lowercase()))
        })
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

pub struct DefaultRowParser {
    pub default_department: String,
    pub default_office: Office,
}

pub struct LastNameFirstParser;

pub struct PharmacyParser;

pub struct ChemicalSciencesParser;

pub struct PhysicsAndAstronomyParser;

pub struct VeterinaryMedicineParser;

pub struct BiologicalSciencesParser;

pub struct StatisticsParser;

impl HtmlRowParser for DefaultRowParser {
    fn parse_department(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        let Some(element) = element else {
            return Some(self.default_department.clone());
        };

        element
            .text()
            .next()
            .and_then(|department_text| Some(department_text.trim().to_string()))
    }

    fn parse_office(&self, element: &Option<ElementRef<'_>>) -> Option<Office> {
        let Some(element) = element else {
            return Some(self.default_office.clone());
        };
        let mut location_text = element.text();
        let Some(location_text_node) = location_text.next() else {
            return Some(self.default_office.clone());
        };
        let mut location = location_text_node.trim().split(" ");

        Some(Office {
            building: location.next().unwrap_or_else(|| "").to_string(),
            room: location.next().unwrap_or_else(|| "").to_string(),
        })
    }
}

impl HtmlRowParser for PharmacyParser {
    fn parse_department(&self, _element: &Option<ElementRef<'_>>) -> Option<String> {
        Some(String::from("School of Pharmacy"))
    }

    fn parse_names(&self, elements: &Vec<ElementRef<'_>>) -> Vec<String> {
        let Some(element) = elements.first() else {
            return vec![];
        };

        match element.text().next() {
            None => vec![],
            Some(text) => text
                .trim()
                .replace("(", "")
                .replace(")", "")
                .split(" ")
                .map(String::from)
                .collect::<Vec<String>>(),
        }
    }
}

impl HtmlRowParser for LastNameFirstParser {
    fn parse_names(&self, elements: &Vec<ElementRef<'_>>) -> Vec<String> {
        let Some(element) = elements.first() else {
            return vec![];
        };

        match element.text().next() {
            None => vec![],
            Some(text) => text
                .trim()
                .split(", ")
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|part| part.split(" "))
                .flatten()
                .map(|part| part.to_string())
                .collect(),
        }
    }
}

impl HtmlRowParser for ChemicalSciencesParser {
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
            room: location.next().unwrap_or_else(|| "").to_string(),
            building: location.next().unwrap_or_else(|| "").to_string(),
        })
    }

    fn parse_department(&self, _element: &Option<ElementRef<'_>>) -> Option<String> {
        Some(String::from("Department Of Chemistry"))
    }

    fn parse_names(&self, elements: &Vec<ElementRef<'_>>) -> Vec<String> {
        let Some(element) = elements.first() else {
            return vec![];
        };

        match element.text().next() {
            Some(text) => text
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
                .collect(),
            None => vec![],
        }
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

    fn parse_names(&self, elements: &Vec<ElementRef<'_>>) -> Vec<String> {
        let Some(element) = elements.first() else {
            return vec![];
        };

        match element.text().next() {
            Some(text) => text
                .trim()
                .split(", ")
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|part| part.split(" "))
                .flatten()
                .map(|part| part.to_string())
                .collect(),
            None => vec![],
        }
    }

    fn parse_id(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        element.and_then(|element| element.text().next().and_then(|id| Some(id.to_string())))
    }

    fn parse_email(&self, element: &Option<ElementRef<'_>>) -> Option<String> {
        self.parse_id(element)
            .and_then(|id| Some(format!("{}@purdue.edu", id)))
    }

    fn parse_department(&self, _element: &Option<ElementRef<'_>>) -> Option<String> {
        Some(String::from("Department of Physics and Astronomy"))
    }

    fn parse_office(&self, element: &Option<ElementRef<'_>>) -> Option<Office> {
        let Some(element) = element else {
            return Some(Office {
                building: String::from("PHYS"),
                room: String::from(""),
            });
        };
        let mut location_text = element.text();
        let Some(location_text_node) = location_text.next() else {
            return Some(Office {
                building: String::from("PHYS"),
                room: String::from(""),
            });
        };
        let mut location = location_text_node.trim().split(" ");

        Some(Office {
            building: location
                .next()
                .map(|building| match building.is_empty() {
                    true => "PHYS",
                    false => building,
                })
                .unwrap_or_else(|| "")
                .to_string(),
            room: location.next().unwrap_or_else(|| "").to_string(),
        })
    }
}

impl HtmlRowParser for VeterinaryMedicineParser {
    fn parse_office(&self, _element: &Option<ElementRef<'_>>) -> Option<Office> {
        Some(Office {
            building: String::from(""),
            room: String::from(""),
        })
    }

    fn parse_department(&self, _element: &Option<ElementRef<'_>>) -> Option<String> {
        Some(String::from("Department of Veterinary Medicine"))
    }

    fn parse_names(&self, elements: &Vec<ElementRef<'_>>) -> Vec<String> {
        let Some(element) = elements.first() else {
            return vec![];
        };

        match element.text().next() {
            Some(text) => text
                .trim()
                .replace("(", "")
                .replace(")", "")
                .replace(".", "")
                .split(", ")
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|part| part.split(" "))
                .flatten()
                .map(|part| part.to_string())
                .collect(),
            None => vec![],
        }
    }
}

impl HtmlRowParser for BiologicalSciencesParser {
    fn parse_department(&self, _element: &Option<ElementRef<'_>>) -> Option<String> {
        Some(String::from("School of Biological sciences"))   
    }

    fn parse_office(&self, element: &Option<ElementRef<'_>>) -> Option<Office> {
        let Some(element) = element else {
            return None;
        };
        let mut location_text = element.text().skip(1);
        let Some(location_text_node) = location_text.next() else {
            return None;
        };
        let cleaned_location = location_text_node
            .replace(" (lab)", "")
            .replace(" (Lab)", "");
        let mut location = cleaned_location.trim().split(" ");

        Some(Office {
            building: location.next().unwrap_or_else(|| "").to_string(),
            room: location.next().unwrap_or_else(|| "").to_string(),
        })
    }
}

impl HtmlRowParser for StatisticsParser {
    fn parse_department(&self, _element: &Option<ElementRef<'_>>) -> Option<String> {
        Some(String::from("Department of Statistics"))
    }

    fn parse_office(&self, element: &Option<ElementRef<'_>>) -> Option<Office> {
        let Some(element) = element else {
            return Some(Office {
                building: String::from("MATH"),
                room: String::from(""),
            });
        };
        let mut location_text = element.text();

        if let Some("Email: ") = location_text.next() {
            return Some(Office {
                building: String::from("MATH"),
                room: String::from(""),
            });
        }

        let Some(location_text_node) = location_text.next() else {
            return Some(Office {
                building: String::from("MATH"),
                room: String::from(""),
            });
        };

        let mut location = location_text_node.trim().split(" ");

        Some(Office {
            building: location
                .next()
                .unwrap_or_else(|| "")
                .replace("Office:", "")
                .trim()
                .to_string(),
            room: location.next().unwrap_or_else(|| "").to_string(),
        })
    }
}
