use csv::Reader;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};

use crate::college::{get_student_by_legal_name, LegalName};

#[derive(Debug)]
pub struct Salary {
    pub student_id: String,
    pub amount_usd: usize,
    pub year: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndianaCompensationRow {
    #[serde(rename = "Year")]
    year: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Department")]
    department: String,
    #[serde(rename = "JobTitle")]
    job_title: String,
    #[serde(rename = "City")]
    city: String,
    #[serde(rename = "TotalCompensation")]
    total_compensation: String,
}

pub fn process_salaries(connection_pool: &Pool<SqliteConnectionManager>) -> Vec<Salary> {
    let mut reader = Reader::from_path("data/perdue_salaries_2023.csv").unwrap();
    let mut salaries = vec![];

    for row in reader.deserialize::<IndianaCompensationRow>() {
        let row = row.unwrap();
        let year: usize = row.year[16..].to_string().parse().unwrap();
        let names: Vec<&str> = row.name.split(", ").collect();
        let legal_name = LegalName {
            legal_first_name: names[1][..names[1].find(" ").unwrap()].to_string(),
            legal_last_name: names[0].replace(",", ""),
        };
        let amount_usd: usize = row
            .total_compensation
            .replace("$", "")
            .replace(",", "")
            .replace(".", "")
            .parse()
            .unwrap();
        let student = get_student_by_legal_name(legal_name, connection_pool);

        if student.is_none() {
            continue;
        }

        salaries.push(Salary {
            student_id: student.unwrap().id,
            amount_usd,
            year,
        })
    }

    salaries
}

pub fn store_salaries(salaries: &Vec<Salary>, connection_pool: &Pool<SqliteConnectionManager>) {
    for salaries_chunk in salaries.chunks(50) {
        let query = salaries_chunk
            .iter()
            .map(|salary| {
                format!(
                    "SELECT '{}' AS StudentId, {} AS Year, {} AS AmountUsd\n",
                    salary.student_id, salary.year, salary.amount_usd
                )
            })
            .collect::<Vec<String>>()
            .join("UNION ALL ");
        connection_pool
            .get()
            .unwrap()
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO Salaries
            (StudentId, Year, AmountUsd) {query}"
                ),
                [],
            )
            .unwrap();
    }
}
