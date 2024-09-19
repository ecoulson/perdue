use std::{io::Cursor, str::FromStr};

use askama::Template;
use num_format::{Buffer, Locale};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Request, Response};

use crate::error::Status;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Office {
    pub building: String,
    pub room: String,
}

#[derive(Clone, Default)]
pub struct College {
    pub id: String,
    pub name: String,
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

#[derive(Template)]
#[template(path = "college_page.html")]
pub struct CollegePage {
    pub college: College,
    pub students: Vec<StudentDirectoryRow>,
}

pub struct StudentDirectoryRow {
    pub id: String,
    pub college_id: String,
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
            "SELECT Id, Department, Email, Name, Office, Year, AmountUsd, CollegeId
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
        dbg!(row);

        directory.push(StudentDirectoryRow {
            id: row.get("Id").unwrap(),
            college_id: row.get("CollegeId").unwrap(),
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

// Renders a page with information about the college and all graduate students in the college
pub fn display_college(
    request: &Request,
    connection_pool: &Pool<SqliteConnectionManager>,
) -> Response<Cursor<Vec<u8>>> {
    let college_id = request.url().split("/college/").skip(1).next().unwrap();
    let connection = connection_pool.get().unwrap();
    let mut college_statement = connection
        .prepare("SELECT Id, Name, Url FROM Colleges WHERE Id = ?1")
        .unwrap();
    let mut students_statement = connection
        .prepare(
            "SELECT Id, Department, Email, Name, Office, Year, AmountUsd, CollegeId
                 FROM Students JOIN Salaries ON Students.Id = Salaries.StudentId AND Students.CollegeId = ?1 ORDER BY Id ASC",
        )
        .unwrap();
    let mut students = vec![];
    let mut student_query = students_statement.query(&[&college_id]).unwrap();
    let college = college_statement
        .query_row(&[&college_id], |row| {
            let mut college = College::default();
            college.name = row.get("Name").unwrap();
            college.id = row.get("Id").unwrap();

            Ok(college)
        })
        .unwrap();

    while let Ok(Some(row)) = student_query.next() {
        let name: String = row.get("Name").unwrap();
        let office_raw: String = row.get("Office").unwrap();
        let year: usize = row.get("Year").unwrap();
        let yearly_compensation: usize = row.get("AmountUsd").unwrap();
        let dollars = yearly_compensation / 100;
        let cents = yearly_compensation % 100;
        let mut compensation_buffer = Buffer::default();
        compensation_buffer.write_formatted(&dollars, &Locale::en);

        students.push(StudentDirectoryRow {
            id: row.get("Id").unwrap(),
            department: row.get("Department").unwrap(),
            college_id: row.get("CollegeId").unwrap(),
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

    Response::from_string(CollegePage { college, students }.to_string())
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
                      '{}' AS Office, '{}' AS CollegeId\n",
                    student.id,
                    student.names.join(" ").replace("'", "''"),
                    student.email.replace("'", "''"),
                    student.department.replace("'", "''"),
                    serde_json::to_string(&student.office).unwrap(),
                    "1"
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
                    "INSERT OR REPLACE INTO Students (Id, Name, Email, Department, Office, CollegeId) {query}"
                ),
                [],
            )
            .unwrap();
    }
}
