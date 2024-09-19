use std::{
    fs::{read_dir, File},
    sync::Arc,
    thread,
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tiny_http::{Request, Response, Server};

use crate::{
    college::{display_college, list_students},
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

fn route(request: Request, state: &Arc<ServerState>) {
    match request.url() {
        "/" => request
            .respond(list_students(&state.connection_pool))
            .unwrap(),
        "/college" => {
            let response = display_college(&request, &state.connection_pool);
            request.respond(response).unwrap()
        }
        _ if request.url().starts_with("/assets") => {
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
