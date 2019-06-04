use std::time::Duration;

use crossbeam::{bounded, Receiver, Sender};
use tokio::prelude::Future;
use tokio::runtime::{Builder, Runtime};

use agent_core::http::body::{IngestBody, LineBuilder};
use agent_core::http::client::Client as HttpClient;
use agent_core::http::request::RequestTemplate;
use agent_core::http::response::Response;

pub struct Client {
    inner: HttpClient,
    runtime: Runtime,
    line_sender: Sender<LineBuilder>,
    line_receiver: Receiver<LineBuilder>,
}

impl Client {
    pub fn new(template: RequestTemplate) -> Self {
        let mut runtime = Builder::new()
            .core_threads(1)
            .build()
            .expect("Runtime::new()");
        let (s, r) = bounded(0);
        Self {
            inner: HttpClient::new(template, &mut runtime),
            runtime,
            line_sender: s,
            line_receiver: r,
        }
    }

    pub fn sender(&self) -> Sender<LineBuilder> {
        self.line_sender.clone()
    }

    pub fn run(mut self) {
        let mut lines = Vec::new();
        let mut lines_bytes = 0;
        loop {
            let lines_to_send = match self.line_receiver.recv_timeout(Duration::from_millis(250)) {
                Ok(line) => {
                    match line.build() {
                        Ok(line) => {
                            lines.push(line);
                            if lines_bytes < 2 * 1024 * 1024 {
                                continue;
                            }
                            lines
                        }
                        Err(_) => { continue; }
                    }
                }
                Err(_) => lines,
            };
            lines = Vec::new();
            lines_bytes = 0;

            if lines_to_send.is_empty() {
                continue
            }

            let fut = self.inner.send(IngestBody::new(lines_to_send))
                .then(|r| {
                    match r {
                        Ok(Response::Failed(_, s, r)) => println!("{},{}", s, r),
                        Err(e) => println!("{}", e),
                        _ => {}
                    }
                    Ok(())
                });
            self.runtime.spawn(fut);
        }
    }
}