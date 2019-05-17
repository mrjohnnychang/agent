use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use crossbeam::{Receiver, Sender, unbounded};

use crate::Event;

pub struct Line {
    pub line: String,
    pub file: String,
    pub length: u64,
}

pub struct Tailer {
    event_sender: Sender<Event>,
    event_receiver: Receiver<Event>,
    offsets: HashMap<PathBuf, u64>,
}

impl Tailer {
    pub fn new() -> Self {
        let (s, r) = unbounded();
        Self {
            event_sender: s,
            event_receiver: r,
            offsets: HashMap::new(),
        }
    }

    pub fn sender(&self) -> Sender<Event> {
        self.event_sender.clone()
    }

    pub fn run(mut self, sender: Sender<Line>) {
        loop {
            let event = self.event_receiver.recv().unwrap();

            match event {
                Event::Initiate(path) => {
                    let len = path.metadata().map(|m| m.len()).unwrap_or(0);
                    info!("initiated {:?} to offset table with offset {}", path, len);
                    self.offsets.insert(path, len);
                }
                Event::New(path) => {
                    info!("added {:?} to offset table", path);
                    self.offsets.insert(path.clone(), 0);
                    self.tail(path, &sender);
                }
                Event::Delete(ref path) => {
                    info!("removed {:?} from offset table", path);
                    self.offsets.remove(path);
                }
                Event::Write(path) => self.tail(path, &sender),
            }
        }
    }

    fn tail(&mut self, path: PathBuf, sender: &Sender<Line>) {
        let offset = match self.offsets.get_mut(&path) {
            Some(v) => v,
            None => {
                warn!("{:?} was not found in offset table!", path);
                return;
            }
        };

        let file_name = path.to_str().unwrap_or("").to_string();
        //todo when match postfix lands on stable replace prefix match for readability
        let (mut reader, len) = match File::open(&path)
            .and_then(|f| f.metadata().map(|m| (f, m)))
            .map(|(f, m)| (BufReader::new(f), m.len())) {
            Ok(v) => v,
            Err(e) => {
                error!("unable to access {:?}: {:?}", path, e);
                return;
            }
        };

        if *offset > len {
            info!("{:?} was truncated from {} to {}", path, offset, len);
            *offset = len;
            return;
        }

        if *offset == len {
            return;
        }

        if let Err(e) = reader.seek(SeekFrom::Start(*offset)) {
            error!("error seeking {:?}", e);
            return;
        }

        loop {
            let mut raw_line = Vec::new();

            let line_len = match reader.read_until(b'\n', &mut raw_line) {
                Ok(v) => v as u64,
                Err(e) => {
                    error!("error reading from file {:?}: {:?}", path, e);
                    return;
                    ;
                }
            };

            let mut line = String::from_utf8(raw_line)
                .unwrap_or_else(|e|
                    String::from_utf8_lossy(e.as_bytes()).to_string()
                );

            if !line.ends_with('\n') {
                return;
            }
            line.pop();

            *offset += line_len;

            sender.send(Line{
                line,
                file: file_name.clone(),
                length: line_len,
            }).unwrap()
        }
    }
}