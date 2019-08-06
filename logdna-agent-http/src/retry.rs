use std::fs::{create_dir_all, File, OpenOptions, read_dir, remove_file};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use chrono::prelude::Utc;
use crossbeam::{Receiver, scope, Sender, unbounded};
use either::Either;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agent_core::http::body::{IngestBody, LineBuilder};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(e: std::io::Error) {
            from()
        }
        Serde(e: serde_json::Error){
            from()
        }
    }
}

pub struct Retry {
    retry_sender: Sender<Arc<IngestBody>>,
    retry_receiver: Receiver<Arc<IngestBody>>,
    line_sender: Sender<Either<LineBuilder, IngestBody>>,
}

#[derive(Deserialize, Serialize)]
struct Wrapper {
    body: IngestBody,
    timestamp: i64,
}

impl Retry {
    pub fn new() -> Retry {
        let (s, r) = unbounded();
        let (temp, _) = unbounded();
        Retry {
            retry_sender: s,
            retry_receiver: r,
            line_sender: temp,
        }
    }

    pub fn sender(&self) -> Sender<Arc<IngestBody>> {
        self.retry_sender.clone()
    }

    pub fn run(mut self, line_sender: Sender<Either<LineBuilder, IngestBody>>) {
        self.line_sender = line_sender;

        create_dir_all("/tmp/logdna/").expect("can't create /tmp/logdna");
        scope(|s| {
            s.spawn(|_| self.poll_incoming());
            s.spawn(|_| self.poll_filesystem());
        }).expect("failed starting Retry")
    }

    fn poll_incoming(&self) {
        loop {
            let body = self.retry_receiver.recv().unwrap();

            let wrapper = match Arc::try_unwrap(body) {
                Ok(v) => v,
                Err(v) => v.as_ref().clone(),
            };

            let err = OpenOptions::new()
                .create(true)
                .write(true)
                .open(format!("/tmp/logdna/{}.retry", Uuid::new_v4().to_string()))
                .map_err(Error::from)
                .and_then(|f| Ok(serde_json::to_writer(f, &wrapper)?));

            if let Err(e) = err {
                error!("retry has failed: {}", e);
            }
        }
    }

    fn poll_filesystem(&self) {
        loop {
            let files = match read_dir("/tmp/logdna/") {
                Ok(v) => v,
                Err(e) => {
                    error!("error reading /tmp/logdna/: {}", e);
                    continue;
                }
            };

            files
                .filter_map(|f| f.ok())
                .map(|f| f.path())
                .filter(|p| p.is_file())
                .filter_map(|p|
                    File::open(&p)
                        .ok()
                        .and_then(|f|
                            serde_json::from_reader::<_, Wrapper>(f)
                                .ok()
                                .map(|w| (p, w))
                        )
                )
                .filter(|(_, w)| Utc::now().timestamp() - w.timestamp > 15)
                .for_each(|(p, w)| {
                    if self.line_sender
                        .send(Either::Right(w.body))
                        .ok()
                        .and_then(|_|
                            remove_file(&p)
                                .ok()
                        )
                        .is_none()
                    {
                        error!("failed deleting retry file {:?}!", p)
                    }
                });

            sleep(Duration::from_secs(15));
        }
    }
}