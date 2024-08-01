use std::{
    io::Cursor,
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
};

use tiny_http::{Response, Server};

pub struct TestServer {
    server: Server,
    sender: Sender<Response<Cursor<Vec<u8>>>>,
}

impl TestServer {
    pub fn new() -> Arc<TestServer> {
        let (sender, receiver) = channel();
        let server = Arc::new(TestServer {
            server: Server::http("0.0.0.0:0").unwrap(),
            sender,
        });
        let test_server = server.clone();

        std::thread::spawn(move || {
            while let Ok(request) = test_server.server.recv() {
                let Ok(response) = receiver.recv() else {
                    request
                        .respond(Response::from_string("No responses queued").with_status_code(500))
                        .unwrap();
                    return;
                };

                request.respond(response).unwrap();
            }
        });

        server
    }

    pub fn add_response(&self, response: Response<Cursor<Vec<u8>>>) {
        self.sender.send(response).unwrap()
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.server.server_addr().to_string())
    }
}
