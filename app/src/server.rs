use std::{
    env::current_dir,
    fs::{read_dir, File},
    sync::Arc,
    thread,
};

use configuration::Configuration;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tiny_http::{Request, Response, Server};

use crate::college::{display_college, list_students};

pub struct ServerState {
    pub connection_pool: Pool<SqliteConnectionManager>,
}

pub fn start_server(configuration: &Configuration, state: Arc<ServerState>) {
    println!("Server is listening");
    let server =
        Arc::new(Server::http(format!("{}:{}", configuration.host, configuration.port)).unwrap());
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
            let response = serve_directory(&request, "/assets", "/assets");
            request.respond(response).unwrap()
        }
        _ => println!("Unhandled route {}", request.url()),
    }
}

fn serve_directory(request: &Request, url: &str, directory_path: &str) -> Response<File> {
    let current_dir = current_dir().unwrap().to_str().unwrap().to_string();
    let resolved_directory_path = format!("{}{}", current_dir, directory_path);

    match read_dir(resolved_directory_path) {
        Ok(directory) => directory
            .filter_map(|file| file.ok())
            .find(|file| {
                file.path()
                    .to_str()
                    .unwrap()
                    .replace(&current_dir, "")
                    .replace(&directory_path, "")
                    == request.url().replace(&url, "")
            })
            .map(|file| Response::from_file(File::open(file.path()).unwrap()))
            .unwrap(),
        Err(_) => panic!("Can't find file"),
    }
}
