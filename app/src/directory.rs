use std::{fmt::Display, io::Cursor, str::FromStr};

use askama::Template;
use num_format::{Buffer, Locale};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Statement};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Request, Response};

use crate::{
    college::Office,
    http::{extract_query, find_header, parse_form_data},
    server::empty_fragment,
};

#[derive(Template)]
#[template(path = "directory.html")]
pub struct Directory {
    pub headings: Vec<DirectoryHeading>,
    pub rows: Vec<StudentDirectoryRow>,
}

#[derive(Template)]
#[template(path = "list_students.html")]
pub struct ListStudents {
    pub directory: Directory,
    pub filters: Vec<DirectoryFilter>,
}

#[derive(Template)]
#[template(path = "directory_filter.html")]
pub struct DirectoryFilter {
    pub column: String,
    pub value: String,
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

#[derive(Template)]
#[template(path = "directory_heading.html")]
pub struct DirectoryHeading {
    column: String,
    formated_column: String,
    sort_state: SortState,
}

#[derive(Template)]
#[template(path = "directory_filter_menu.html")]
pub struct DirectoryFilterMenu {
    pub columns: Vec<Column>,
}

pub struct Column {
    pub name: String,
    pub formatted_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DirectoryQuery {
    filters: Option<Vec<String>>,
    sort_column: Option<String>,
    sort_direction: Option<SortDirection>,
}

#[derive(Deserialize, Debug)]
struct CreateDirectoryFilterRequest {
    column: String,
    value: String,
}

#[derive(Deserialize, Debug)]
struct CreateDirectorySortRequest {
    column: String,
    state: SortState,
}

#[derive(Deserialize, Serialize, Debug)]
enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Deserialize, Serialize, Debug)]
enum SortState {
    Unsorted,
    Ascending,
    Descending,
}

impl Display for SortState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl SortDirection {
    pub fn to_sql(&self) -> String {
        match &self {
            SortDirection::Ascending => String::from("ASC"),
            SortDirection::Descending => String::from("DESC"),
        }
    }
}

pub fn sort_directory(request: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let current_url = find_header(request, "HX-Current-Url").unwrap();
    let mut query: DirectoryQuery = extract_query(current_url.value.as_str()).unwrap();
    let sort: CreateDirectorySortRequest = parse_form_data(request).unwrap();

    query.sort_column = Some(sort.column);
    query.sort_direction = Some(match sort.state {
        SortState::Unsorted => SortDirection::Ascending,
        SortState::Ascending => SortDirection::Descending,
        SortState::Descending => SortDirection::Ascending,
    });

    empty_fragment()
        .with_header(Header::from_str("Content-Type: text/html").unwrap())
        .with_header(
            Header::from_str(&format!(
                "HX-Push-Url: /?{}",
                serde_qs::to_string(&query).unwrap()
            ))
            .unwrap(),
        )
        .with_header(Header::from_str("HX-Trigger: close-directory-filter-menu").unwrap())
        .with_header(Header::from_str("HX-Trigger-After-Settle: filter-directory").unwrap())
}

pub fn fetch_columns() -> Vec<Column> {
    vec![
        Column {
            name: String::from("Id"),
            formatted_name: String::from("Id"),
        },
        Column {
            name: String::from("Name"),
            formatted_name: String::from("Name"),
        },
        Column {
            name: String::from("Email"),
            formatted_name: String::from("Email"),
        },
        Column {
            name: String::from("Department"),
            formatted_name: String::from("Department"),
        },
        Column {
            name: String::from("Building"),
            formatted_name: String::from("Building"),
        },
        Column {
            name: String::from("Room"),
            formatted_name: String::from("Room"),
        },
        Column {
            name: String::from("AmountUsd"),
            formatted_name: String::from("Yearly Compensation"),
        },
        Column {
            name: String::from("Year"),
            formatted_name: String::from("Year"),
        },
    ]
}

pub fn build_directory_filter_menu() -> Response<Cursor<Vec<u8>>> {
    Response::from_string(
        DirectoryFilterMenu {
            columns: fetch_columns(),
        }
        .to_string(),
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap())
}

pub fn delete_directory_filter(request: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let current_url = find_header(request, "HX-Current-Url").unwrap();
    let mut query: DirectoryQuery = extract_query(current_url.value.as_str()).unwrap();
    let filter: CreateDirectoryFilterRequest = parse_form_data(request).unwrap();

    if let Some(filters) = query.filters.as_mut() {
        if let Some(index) = filters
            .iter()
            .position(|query_filter| query_filter == &format!("{}={}", filter.column, filter.value))
        {
            filters.remove(index);
        }
    }

    empty_fragment()
        .with_header(Header::from_str("HX-Trigger-After-Settle: filter-directory").unwrap())
        .with_header(
            Header::from_str(&format!(
                "HX-Push-Url: /?{}",
                serde_qs::to_string(&query).unwrap()
            ))
            .unwrap(),
        )
}

pub fn create_directory_filter(request: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let filter: CreateDirectoryFilterRequest = parse_form_data(request).unwrap();
    let current_url = find_header(&request, "HX-Current-Url").unwrap();
    let mut query: DirectoryQuery = extract_query(current_url.value.as_str()).unwrap();
    let serialized_filter = format!("{}={}", filter.column, filter.value);

    if let Some(filters) = query.filters.as_mut() {
        filters.push(serialized_filter);
    } else {
        query.filters = Some(vec![serialized_filter]);
    }

    Response::from_string(
        DirectoryFilter {
            column: filter.column.clone(),
            value: filter.value.clone(),
        }
        .to_string(),
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap())
    .with_header(
        Header::from_str(&format!(
            "HX-Push-Url: /?{}",
            serde_qs::to_string(&query).unwrap()
        ))
        .unwrap(),
    )
    .with_header(Header::from_str("HX-Trigger: close-directory-filter-menu").unwrap())
    .with_header(Header::from_str("HX-Trigger-After-Settle: filter-directory").unwrap())
}

pub fn build_directory(
    request: &Request,
    connection_pool: &Pool<SqliteConnectionManager>,
) -> Response<Cursor<Vec<u8>>> {
    let connection = connection_pool.get().unwrap();
    let url = find_header(request, "HX-Current-Url").unwrap();
    let query: DirectoryQuery = extract_query(url.value.as_str()).unwrap();

    Response::from_string(
        Directory {
            headings: build_headings(&query, &fetch_columns()),
            rows: build_rows(prepare_directory_statement(&query, &connection))
                .into_iter()
                .collect(),
        }
        .to_string(),
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap())
}

fn prepare_directory_statement<'a>(
    query: &DirectoryQuery,
    connection: &'a Connection,
) -> Statement<'a> {
    let condition: String = query
        .filters
        .as_ref()
        .map(|filter| {
            filter
                .iter()
                .map(|filter| {
                    let mut parts = filter.split("=");
                    let column = parts.next().unwrap();
                    let value = parts.next().unwrap();

                    format!("{} = '{}'", column, value)
                })
                .collect::<Vec<String>>()
                .join(" OR ")
        })
        .unwrap_or(String::new());
    let sort = format!(
        "ORDER BY {} {}",
        query.sort_column.as_ref().unwrap_or(&String::from("Id")),
        query
            .sort_direction
            .as_ref()
            .unwrap_or(&SortDirection::Ascending)
            .to_sql()
    );

    if condition.is_empty() {
        return connection
            .prepare(&format!(
                "SELECT Id, Department, Email, Name, Year, AmountUsd, CollegeId, Building, Room
                 FROM Students 
                 JOIN Salaries 
                 ON Students.Id = Salaries.StudentId 
                 LEFT JOIN Offices
                 ON Students.Id = Offices.StudentId 
                 {}",
                sort
            ))
            .unwrap();
    }

    connection
        .prepare(&format!(
            "SELECT Id, Department, Email, Name, Year, AmountUsd, CollegeId, Building, Room
                 FROM Students 
                 JOIN Salaries 
                 ON Students.Id = Salaries.StudentId 
                 LEFT JOIN Offices
                 ON Students.Id = Offices.StudentId
                 WHERE {} {}",
            condition, sort
        ))
        .unwrap()
}

fn build_rows(mut statement: Statement) -> Vec<StudentDirectoryRow> {
    let mut query = statement.query([]).unwrap();
    let mut directory = Vec::new();

    while let Ok(Some(row)) = query.next() {
        let name: String = row.get("Name").unwrap();
        let year: usize = row.get("Year").unwrap();
        let yearly_compensation: usize = row.get("AmountUsd").unwrap();
        let dollars = yearly_compensation / 100;
        let cents = yearly_compensation % 100;
        let mut compensation_buffer = Buffer::default();
        compensation_buffer.write_formatted(&dollars, &Locale::en);

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
            office: Office {
                building: row.get("Building").unwrap_or(String::new()),
                room: row.get("Room").unwrap_or(String::new()),
            },
            yearly_compensation: format!("${}.{}", compensation_buffer.to_string(), cents),
            year,
        });
    }

    directory
}

pub fn list_students(
    request: &Request,
    connection_pool: &Pool<SqliteConnectionManager>,
) -> Response<Cursor<Vec<u8>>> {
    let connection = connection_pool.get().unwrap();
    let query: DirectoryQuery = extract_query(request.url()).unwrap();
    let filters: Vec<DirectoryFilter> = query
        .filters
        .as_ref()
        .map(|filter| {
            filter
                .iter()
                .map(|filter| {
                    let mut parts = filter.split("=");
                    let column = parts.next().unwrap();
                    let value = parts.next().unwrap();

                    DirectoryFilter {
                        column: column.to_string(),
                        value: value.to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or(vec![]);

    Response::from_string(
        ListStudents {
            directory: Directory {
                headings: build_headings(&query, &fetch_columns()),
                rows: build_rows(prepare_directory_statement(&query, &connection)),
            },
            filters,
        }
        .to_string(),
    )
    .with_header(Header::from_str("Content-Type: text/html").unwrap())
}

fn build_headings(query: &DirectoryQuery, columns: &Vec<Column>) -> Vec<DirectoryHeading> {
    columns
        .iter()
        .map(|column| {
            let state = if let Some(sort_column) = query.sort_column.as_ref() {
                if &column.name == sort_column {
                    match &query.sort_direction.as_ref().unwrap() {
                        SortDirection::Descending => SortState::Descending,
                        SortDirection::Ascending => SortState::Ascending,
                    }
                } else {
                    SortState::Unsorted
                }
            } else {
                if column.name == "Id" {
                    SortState::Ascending
                } else {
                    SortState::Unsorted
                }
            };

            DirectoryHeading {
                column: column.name.clone(),
                formated_column: column.formatted_name.clone(),
                sort_state: state,
            }
        })
        .collect()
}
