use std::mem::replace;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam::{after, Receiver, Sender, unbounded};
use either::Either;
use tokio::prelude::Future;
use tokio::runtime::Runtime;

use crate::types::body::{IngestBody, Line, LineBuilder};
use crate::types::client::Client as HttpClient;
use crate::types::error::HttpError;
use crate::types::request::RequestTemplate;
use crate::types::response::Response;

/// Http(s) client used to send logs to the Ingest API
pub struct Client {
    inner: HttpClient,
    runtime: Runtime,
    line_sender: Sender<Either<LineBuilder, IngestBody>>,
    line_receiver: Receiver<Either<LineBuilder, IngestBody>>,
    retry_sender: Sender<Arc<IngestBody>>,

    buffer: Vec<Line>,
    buffer_max_size: usize,
    buffer_bytes: usize,
    buffer_timeout: Receiver<Instant>,
}

impl Client {
    /// Used to create a new instance of client, requiring a channel sender for retry
    /// and a request template for building ingest requests
    pub fn new(template: RequestTemplate) -> Self {
        let mut runtime = Runtime::new().expect("Runtime::new()");
        let (s, r) = unbounded();
        let (temp, _) = unbounded();
        Self {
            inner: HttpClient::new(template, &mut runtime),
            runtime,
            line_sender: s,
            line_receiver: r,
            retry_sender: temp,

            buffer: Vec::new(),
            buffer_max_size: 2 * 1024 * 1024,
            buffer_bytes: 0,
            buffer_timeout: after(Duration::from_millis(250)),
        }
    }
    /// Returns the channel sender used to send data from other threads
    pub fn sender(&self) -> Sender<Either<LineBuilder, IngestBody>> {
        self.line_sender.clone()
    }
    /// The main logic loop, consumes self because it should only be called once
    pub fn run(mut self, retry_sender: Sender<Arc<IngestBody>>) {
        self.retry_sender = retry_sender;

        loop {
            if self.buffer_bytes < self.buffer_max_size {
                let msg = select! {
                    recv(self.line_receiver) -> msg => msg,
                    recv(self.buffer_timeout) -> _ => {
                        self.flush();
                        continue;
                    },
                };
                // The left hand side of the either is new lines the come from the Tailer
                // The right hand side of the either is ingest bodies that are ready for retry
                match msg {
                    Ok(Either::Left(line)) => {
                        if let Ok(line) = line.build() {
                            self.buffer_bytes += line.line.len();
                            self.buffer.push(line);
                        }
                    }
                    Ok(Either::Right(body)) => {
                        self.send(body);
                    }
                    Err(_) => {}
                };
            }

            self.flush()
        }
    }

    pub fn set_max_buffer_size(&mut self, size: usize) {
        self.buffer_max_size = size;
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.inner.set_timeout(timeout)
    }

    fn flush(&mut self) {
        let buffer = replace(&mut self.buffer, Vec::new());
        self.buffer_bytes = 0;
        self.buffer_timeout = new_timeout();

        if buffer.is_empty() {
            return;
        }

        self.send(IngestBody::new(buffer));
    }

    fn send(&mut self, body: IngestBody) {
        let sender = self.retry_sender.clone();
        let fut = self.inner.send(body)
            .then(move |r| {
                match r {
                    Ok(Response::Failed(_, s, r)) => warn!("bad response {}: {}", s, r),
                    Err(HttpError::Send(body, e)) => {
                        warn!("failed sending http request, retrying: {}", e);
                        sender.send(body).unwrap();
                    }
                    Err(e) => {
                        warn!("failed sending http request: {}", e);
                    }
                    Ok(Response::Sent) => {} //success
                };
                Ok(())
            });
        self.runtime.spawn(fut);
    }
}

fn new_timeout() -> Receiver<Instant> {
    after(Duration::from_millis(250))
}