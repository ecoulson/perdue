use std::{
    fs::{read_dir, File},
    io::{Cursor, Read},
    str::FromStr,
    sync::Arc,
    thread,
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::{
    college::display_college,
    configuration::Configuration,
    directory::{
        build_directory, build_directory_filter_menu, create_directory_filter,
        delete_directory_filter, list_students, sort_directory,
    },
};

pub struct ServerState {
    pub connection_pool: Pool<SqliteConnectionManager>,
    pub configuration: Configuration,
}

pub fn start_server(state: Arc<ServerState>) {
    println!("Server is listening");
    let server = Arc::new(
        Server::http(format!(
            "{}:{}",
            state.configuration.host, state.configuration.port
        ))
        .unwrap(),
    );
    let mut workers = Vec::with_capacity(4);

    for _ in 0..workers.capacity() {
        let server = server.clone();
        let state = state.clone();

        workers.push(thread::spawn(move || loop {
            match server.recv() {
                Ok(mut request) => {
                    let response = route(&mut request, &state);
                    request.respond(response).unwrap();
                }
                Err(error) => {
                    eprintln!("error: {}", error)
                }
            }
        }));
    }
}

fn remove_query(url: &str) -> &str {
    url.split("?").next().unwrap()
}

fn get_route_key(request: &Request) -> (&Method, &str) {
    (request.method(), remove_query(request.url()))
}

// PERF NOTE: We are using dynamic dispatch it is slower with Box<dyn Read + Send>
// can swap to an enum to wrap the type if this is a bottleneck
fn route(request: &mut Request, state: &Arc<ServerState>) -> Response<Box<dyn Read + Send>> {
    match get_route_key(request) {
        (Method::Get, "/") => list_students(&request, &state).boxed(),
        (Method::Get, "/college") => display_college(&request, &state).boxed(),
        (Method::Get, "/directory") => build_directory(&request, &state).boxed(),
        (Method::Delete, "/remove_directory_filter") => delete_directory_filter(request).boxed(),
        (Method::Get, "/directory_filter_menu") => build_directory_filter_menu().boxed(),
        (Method::Post, "/create_directory_filter") => create_directory_filter(request).boxed(),
        (Method::Post, "/sort_directory") => sort_directory(request).boxed(),
        (Method::Get, "/member") if request.url().starts_with("/member") => Response::from_string("epically in progress")
            .with_status_code(StatusCode::from(200))
            .boxed(),
        (Method::Get, _) if request.url().starts_with("/assets") => serve_directory(
            &request,
            "/assets",
            &state.configuration.files.assets_directory,
        )
        .boxed(),
        _ => {
            println!("Unhandled route {}", request.url());
            Response::empty(StatusCode::from(404)).boxed()
        }
    }
}

pub fn empty_fragment() -> Response<Cursor<Vec<u8>>> {
    Response::from_string("").with_header(Header::from_str("Content-Type: text/html").unwrap())
}

fn serve_directory(request: &Request, url: &str, directory_path: &str) -> Response<File> {
    match read_dir(directory_path) {
        Ok(directory) => directory
            .filter_map(|file| file.ok())
            .find(|file| {
                file.path().to_str().unwrap().replace(&directory_path, "")
                    == request.url().replace(&url, "")
            })
            .map(|file| Response::from_file(File::open(file.path()).unwrap()))
            .unwrap(),
        Err(_) => panic!("Can't find file {}", url),
    }
}
