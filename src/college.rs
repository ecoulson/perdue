use askama::Template;
use axum::response::IntoResponse;
use num_format::{Buffer, Locale};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};

use crate::{database::DatabaseConnection, template::HtmlTemplate};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Office {
    pub building: String,
    pub room: String,
}

#[derive(Clone)]
pub struct College {
    pub base_url: String,
    pub default_office: Office,
    pub default_department: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraduateStudent {
    pub id: String,
    pub names: Vec<String>,
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
        let yearly_compensation: usize = row.get("AmountUsd").unwrap();
        let dollars = yearly_compensation / 100;
        let cents = yearly_compensation % 100;
        let mut compensation_buffer = Buffer::default();
        compensation_buffer.write_formatted(&dollars, &Locale::en);

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
            yearly_compensation: format!("${}.{}", compensation_buffer.to_string(), cents),
            year,
        });
    }

    HtmlTemplate {
        template: ListStudents { directory },
    }
}

pub fn get_student_by_name(
    names: &Vec<String>,
    connection_pool: &Pool<SqliteConnectionManager>,
) -> Option<GraduateStudent> {
    let connection = connection_pool.get().unwrap();
    let mut names = names.clone();
    let mut name = names.join("%").replace("'", "''");
    let mut student = connection
        .query_row(
            "SELECT Id, Email, Name, Office, Department FROM Students WHERE Name LIKE ?1",
            &[&name],
            |row| {
                let name: String = row.get("Name").unwrap();
                let office_raw: String = row.get("Office").unwrap();

                Ok(GraduateStudent {
                    id: row.get("Id").unwrap(),
                    department: row.get("Department").unwrap(),
                    email: row.get("Email").unwrap(),
                    names: name.split(", ").map(|part| part.to_string()).collect(),
                    office: serde_json::from_str(&office_raw).unwrap(),
                })
            },
        )
        .ok();

    while student.is_none() && names.len() > 2 {
        names.remove(1);
        name = names.join("%").replace("'", "''");
        student = connection
            .query_row(
                "SELECT Id, Email, Name, Office, Department FROM Students WHERE Name LIKE ?1",
                &[&name],
                |row| {
                    let name: String = row.get("Name").unwrap();
                    let office_raw: String = row.get("Office").unwrap();

                    Ok(GraduateStudent {
                        id: row.get("Id").unwrap(),
                        department: row.get("Department").unwrap(),
                        email: row.get("Email").unwrap(),
                        names: name.split(", ").map(|part| part.to_string()).collect(),
                        office: serde_json::from_str(&office_raw).unwrap(),
                    })
                },
            )
            .ok();
    }

    student
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
                      '{}' AS Email, '{}' AS Department,
                      '{}' AS Office\n",
                    student.id,
                    student.names.join(" ").replace("'", "''"),
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
                    "INSERT OR REPLACE INTO Students (Id, Name, Email, Department, Office) {query}"
                ),
                [],
            )
            .unwrap();
    }
}
