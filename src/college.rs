use askama::Template;
use axum::response::IntoResponse;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};

use crate::{database::DatabaseConnection, html::HtmlTemplate};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Office {
    pub building: String,
    pub room: String,
}

pub struct College {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduateStudent {
    pub id: String,
    pub name: Vec<String>,
    pub legal_first_name: String,
    pub legal_last_name: String,
    pub email: String,
    pub department: String,
    pub office: Office,
}

#[derive(Template)]
#[template(path = "list_students.html")]
pub struct ListStudents {
    pub directory: Vec<StudentDirectoryRow>,
}

pub struct StudentDirectoryRow {
    pub id: String,
    pub department: String,
    pub email: String,
    pub name: String,
    pub office: Office,
    pub yearly_compensation: String,
    pub year: usize,
}

pub async fn list_students(
    DatabaseConnection(connection): DatabaseConnection,
) -> impl IntoResponse {
    let mut statement = connection
        .prepare(
            "SELECT Id, Department, Email, Name, Office, Year, AmountUsd 
                 FROM Students JOIN Salaries ON Students.Id = Salaries.StudentId ORDER BY Id ASC",
        )
        .unwrap();
    let mut query = statement.query([]).unwrap();
    let mut directory = Vec::new();

    while let Ok(Some(row)) = query.next() {
        let name: String = row.get("Name").unwrap();
        let office_raw: String = row.get("Office").unwrap();
        let year: usize = row.get("Year").unwrap();
        let yearly_compensation: f64 = row.get("AmountUsd").unwrap();

        directory.push(StudentDirectoryRow {
            id: row.get("Id").unwrap(),
            department: row.get("Department").unwrap(),
            email: row.get("Email").unwrap(),
            name: name
                .split(", ")
                .map(|part| part.to_string())
                .collect::<Vec<String>>()
                .join(" "),
            office: serde_json::from_str(&office_raw).unwrap(),
            yearly_compensation: format!("${:.2}", yearly_compensation / 100.0),
            year,
        });
    }

    HtmlTemplate {
        template: ListStudents { directory },
    }
}

#[derive(Debug)]
pub struct LegalName {
    pub legal_first_name: String,
    pub legal_last_name: String,
}

pub fn get_student_by_legal_name(
    legal_name: LegalName,
    connection_pool: &Pool<SqliteConnectionManager>,
) -> Option<GraduateStudent> {
    let connection = connection_pool.get().unwrap();

    connection
        .query_row(
            "SELECT * FROM Students WHERE LegalFirstName = ?1 AND LegalLastName = ?2",
            (&legal_name.legal_first_name, &legal_name.legal_last_name),
            |row| {
                let name: String = row.get("Name").unwrap();
                let office_raw: String = row.get("Office").unwrap();

                Ok(GraduateStudent {
                    id: row.get("Id").unwrap(),
                    department: row.get("Department").unwrap(),
                    email: row.get("Email").unwrap(),
                    legal_last_name: row.get("LegalLastName").unwrap(),
                    legal_first_name: row.get("LegalFirstName").unwrap(),
                    name: name.split(", ").map(|part| part.to_string()).collect(),
                    office: serde_json::from_str(&office_raw).unwrap(),
                })
            },
        )
        .ok()
}

pub fn store_students(
    students: &Vec<GraduateStudent>,
    connection_pool: &Pool<SqliteConnectionManager>,
) {
    for students_chunk in students.chunks(50) {
        let query = students_chunk
            .iter()
            .map(|student| {
                format!(
                    "SELECT '{}' AS Id, '{}' AS Name,
                                                      '{}' LegalFirstName, '{}' LegalLastName,
                                                      '{}' AS Email, '{}' AS Department,
                                                      '{}' AS Office\n",
                    student.id,
                    student.name.join(", ").replace("'", "''"),
                    student.legal_first_name.replace("'", "''"),
                    student.legal_last_name.replace("'", "''"),
                    student.email.replace("'", "''"),
                    student.department,
                    serde_json::to_string(&student.office).unwrap()
                )
            })
            .collect::<Vec<String>>()
            .join("UNION ALL ");
        connection_pool
            .get()
            .unwrap()
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO Students 
            (Id, Name, LegalFirstName, LegalLastName, Email, Department, Office) {query}"
                ),
                [],
            )
            .unwrap();
    }
}
