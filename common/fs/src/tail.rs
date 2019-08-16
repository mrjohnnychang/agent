use std::fs::{File, metadata};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use hashbrown::HashMap;
use crossbeam::{bounded, Receiver, Sender};
use either::Either;

use http::types::body::IngestBody;
use http::types::body::LineBuilder;

use crate::Event;

/// Tails files on a filesystem by inheriting events from a Watcher
pub struct Tailer {
    // the sender that we are going to share to threads that want to communicate e.g watcher thread
    event_sender: Sender<Event>,
    // used to pops items out of the sender
    event_receiver: Receiver<Event>,
    // tracks the offset (bytes from the beginning of the file we have read) of file(s)
    offsets: HashMap<PathBuf, u64>,
}

impl Tailer {
    /// Creates new instance of Tailer
    pub fn new() -> Self {
        let (s, r) = bounded(32_000);
        Self {
            event_sender: s,
            event_receiver: r,
            offsets: HashMap::new(),
        }
    }
    /// Returns the sender the tailer is "listening" on
    pub fn sender(&self) -> Sender<Event> {
        self.event_sender.clone()
    }
    /// Runs the main logic of the tailer, this can only be run once so Tailer is consumed
    pub fn run(mut self, sender: Sender<Either<LineBuilder, IngestBody>>) {
        loop {
            // safe to unwrap
            let event = self.event_receiver.recv().unwrap();

            match event {
                Event::Initiate(path) => {
                    // will initiate a file to it's current length
                    let len = path.metadata().map(|m| m.len()).unwrap_or(0);
                    info!("initiated {:?} to offset table with offset {}", path, len);
                    self.offsets.insert(path, len);
                }
                Event::New(path) => {
                    // similar to initiate but sets the offset to 0
                    info!("added {:?} to offset table", path);
                    self.offsets.insert(path.clone(), 0);
                    self.tail(path, &sender);
                }
                Event::Delete(ref path) => {
                    // just remove the file from the offset table on delete
                    // this acts almost like a garbage collection mechanism
                    // ensuring the offset table doesn't "leak" by holding deleted files
                    info!("removed {:?} from offset table", path);
                    self.offsets.remove(path);
                }
                Event::Write(path) => self.tail(path, &sender),
            }
        }
    }

    // tail a file for new line(s)
    fn tail(&mut self, path: PathBuf, sender: &Sender<Either<LineBuilder, IngestBody>>) {
        // get the offset from the map, return if not found
        let offset = match self.offsets.get_mut(&path) {
            Some(v) => v,
            None => {
                warn!("{:?} was not found in offset table!", path);
                return;
            }
        };
        // get the file len
        let len = match metadata(&path).map(|m| m.len()) {
            Ok(v) => v,
            Err(e) => {
                error!("unable to stat {:?}: {:?}", path, e);
                return;
            }
        };
        // if we are at the end of the file there's no work to do
        if *offset == len {
            return;
        }
        // get the name of the file set to "" if the file is invalid utf8
        let file_name = path.to_str().unwrap_or("").to_string();
        // open the file, create a reader
        //todo when match postfix lands on stable replace prefix match for readability
        let mut reader = match File::open(&path)
            .map(|f| BufReader::new(f)) {
            Ok(v) => v,
            Err(e) => {
                error!("unable to access {:?}: {:?}", path, e);
                return;
            }
        };
        // if the offset is greater than the file's len
        // it's very likely a truncation occurred
        if *offset > len {
            info!("{:?} was truncated from {} to {}", path, *offset, len);
            *offset = len;
            return;
        }
        // seek to the offset, this creates the "tailing" effect
        if let Err(e) = reader.seek(SeekFrom::Start(*offset)) {
            error!("error seeking {:?}", e);
            return;
        }

        loop {
            let mut raw_line = Vec::new();
            // read until a new line returning the line length
            let line_len = match reader.read_until(b'\n', &mut raw_line) {
                Ok(v) => v as u64,
                Err(e) => {
                    error!("error reading from file {:?}: {:?}", path, e);
                    return;
                }
            };
            // try to parse the raw data as utf8
            // if that fails replace invalid chars with blank chars
            // see String::from_utf8_lossy docs
            let mut line = String::from_utf8(raw_line)
                .unwrap_or_else(|e|
                    String::from_utf8_lossy(e.as_bytes()).to_string()
                );
            // if the line doesn't end with a new line we might have read in the middle of a write
            // so we return in this case
            if !line.ends_with('\n') {
                return;
            }
            // remove the trailing new line
            line.pop();
            // increment the offset
            *offset += line_len;
            // send the line upstream, safe to unwrap
            sender.send(
                Either::Left(
                    LineBuilder::new()
                        .line(line)
                        .file(file_name.clone())
                )
            ).unwrap()
        }
    }
}