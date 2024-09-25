use std::{
    fs::{read_dir, File},
    io::Cursor,
    str::FromStr,
    sync::Arc,
    thread,
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::{
    college::{
        build_directory, build_directory_filter_menu, create_directory_filter,
        delete_directory_filter, display_college, list_students,
    },
    configuration::Configuration,
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
                Ok(request) => route(request, &state),
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

fn route(mut request: Request, state: &Arc<ServerState>) {
    match get_route_key(&request) {
        (Method::Get, "/") => {
            let response = list_students(&request, &state.connection_pool);
            request.respond(response).unwrap();
        }
        (Method::Get, "/college") => {
            let response = display_college(&request, &state.connection_pool);
            request.respond(response).unwrap()
        }
        (Method::Get, "/directory") => {
            let response = build_directory(&request, &state.connection_pool);
            request.respond(response).unwrap()
        }
        (Method::Delete, "/remove_directory_filter") => {
            let response = delete_directory_filter(&mut request);
            request.respond(response).unwrap()
        }
        (Method::Get, "/directory_filter_menu") => {
            request.respond(build_directory_filter_menu()).unwrap()
        }
        (Method::Post, "/create_directory_filter") => {
            let response = create_directory_filter(&mut request);
            request.respond(response).unwrap()
        }
        (Method::Get, "/empty_fragment") => request.respond(empty_fragment()).unwrap(),
        (Method::Get, _) if request.url().starts_with("/assets") => {
            let response = serve_directory(
                &request,
                "/assets",
                &state.configuration.files.assets_directory,
            );
            request.respond(response).unwrap()
        }
        _ => println!("Unhandled route {}", request.url()),
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
