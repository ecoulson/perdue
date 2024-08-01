use std::{io::Cursor, str::FromStr};

use askama::Template;
use num_format::{Buffer, Locale};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Response};

use crate::error::Status;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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

pub fn list_students(connection_pool: &Pool<SqliteConnectionManager>) -> Response<Cursor<Vec<u8>>> {
    let connection = connection_pool.get().unwrap();
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

    Response::from_string(ListStudents { directory }.to_string())
        .with_header(Header::from_str("Content-Type: text/html").unwrap())
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
    students: &Vec<Result<GraduateStudent, Status>>,
    connection_pool: &Pool<SqliteConnectionManager>,
) {
    for students_chunk in students.chunks(50) {
        let query = students_chunk
            .iter()
            .filter_map(|student| match student {
                Ok(student) => Some(format!(
                    "SELECT '{}' AS Id, '{}' AS Name,
                      '{}' AS Email, '{}' AS Department,
                      '{}' AS Office\n",
                    student.id,
                    student.names.join(" ").replace("'", "''"),
                    student.email.replace("'", "''"),
                    student.department,
                    serde_json::to_string(&student.office).unwrap()
                )),
                Err(error) => {
                    eprintln!("{}", error);
                    None
                }
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
