#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate quick_error;

use std::ffi::OsStr;
use std::fs::{canonicalize, read_dir};
use std::io;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Command;

use chashmap::CHashMap;
use inotify::{EventMask, Inotify, WatchMask};
use regex::Regex;
use serde::{Deserialize, Serialize};

use http::types::body::{KeyValueMap, LineBuilder};
use middleware::{Middleware, Status};

lazy_static! {
    static ref K8S_REG: Regex = Regex::new(
        r#"^/var/log/containers/([a-z0-9A-Z\-.]+)_([a-z0-9A-Z\-.]+)_([a-z0-9A-Z\-.]+)-([a-z0-9]{64}).log$"#
    ).expect("Regex::new()");
}

quick_error! {
    #[derive(Debug)]
    enum Error {
        Io(e: std::io::Error) {
            from()
            display("{}", e)
        }
        Utf(e: std::string::FromUtf8Error) {
            from()
            display("{}", e)
        }
        Regex {
            from()
            display("failed to parse path")
        }
        Serde(e: serde_json::Error){
            from()
            display("{}", e)
        }
    }
}

pub struct K8s {
    real_to_symlinks: CHashMap<PathBuf, PathBuf>,
    symlinks_to_real: CHashMap<PathBuf, PathBuf>,

    labels: CHashMap<PathBuf, KeyValueMap>,
    annotations: CHashMap<PathBuf, KeyValueMap>,
}

impl K8s {
    pub fn new() -> Self {
        K8s {
            real_to_symlinks: CHashMap::new(),
            symlinks_to_real: CHashMap::new(),
            labels: CHashMap::new(),
            annotations: CHashMap::new(),
        }
    }

    fn create_inotify(&self) -> io::Result<Inotify> {
        for file in read_dir("/var/log/containers")? {
            if let Ok(file) = file {
                let symlink = file.path();
                if symlink.is_dir() {
                    continue;
                }

                if let Ok(real) = canonicalize(&symlink) {
                    info!("rewriting {:?} to {:?}", real, symlink);
                    self.real_to_symlinks.insert(real.clone(), symlink.clone());
                    self.symlinks_to_real.insert(symlink.clone(), real.clone());
                    if let Err(e) = self.update_k8s_meta(symlink) {
                        error!("error updating k8s meta: {}", e)
                    }
                }
            };
        }

        let mut inotify = Inotify::init()?;
        inotify.add_watch("/var/log/containers/", WatchMask::CREATE | WatchMask::DELETE | WatchMask::DONT_FOLLOW)?;
        Ok(inotify)
    }

    fn update_k8s_meta(&self, symlink: PathBuf) -> Result<(), Error> {
        let str = symlink.to_str().ok_or(Error::Regex)?;
        let captures = K8S_REG.captures(str).ok_or(Error::Regex)?;
        let name = captures.get(1).ok_or(Error::Regex)?.as_str();
        let namespace = captures.get(2).ok_or(Error::Regex)?.as_str();

        let out = Command::new("kubectl")
            .args(&["get", "pods", "-o", "json", "-n", namespace, name])
            .output()?
            .stdout;
        let out = String::from_utf8(out)?;

        let pod: Pod = serde_json::from_str(&out)?;

        self.labels.insert(symlink.clone(), pod.metadata.labels);
        self.annotations.insert(symlink.clone(), pod.metadata.annotations);

        Ok(())
    }
}

impl Middleware for K8s {
    fn run(&self) {
        let mut inotify = self.create_inotify().expect("Inotify::create()");

        let mut buff = [0u8; 8_192];
        loop {
            let events = match inotify.read_events_blocking(&mut buff) {
                Ok(v) => v,
                Err(_) => {
                    continue;
                }
            };

            for event in events {
                if event.mask.contains(EventMask::CREATE) {
                    if let Some((symlink, real)) = handle_event_name(event.name) {
                        self.symlinks_to_real.insert(symlink.clone(), real.clone());
                        self.real_to_symlinks.insert(real, symlink.clone());
                        if let Err(e) = self.update_k8s_meta(symlink) {
                            error!("error updating k8s meta: {}", e)
                        }
                    }
                } else if event.mask.contains(EventMask::DELETE) {
                    if let Some((symlink, real)) = handle_event_name(event.name) {
                        self.symlinks_to_real.remove(&symlink);
                        self.real_to_symlinks.remove(&real);
                        self.labels.remove(&symlink);
                        self.annotations.remove(&symlink);
                    }
                }
            }
        }
    }

    fn process(&self, mut line: LineBuilder) -> Status {
        if let Some(ref file) = line.file {
            let real = PathBuf::from(file);
            let symlink = self.real_to_symlinks.get(&real);
            if let Some(symlink) = symlink {
                if let Some(file) = symlink.to_str() {
                    line = line.file(file);
                }
                if let Some(labels) = self.labels.get(symlink.deref()) {
                    line = line.labels(labels.clone());
                }
                if let Some(annotations) = self.annotations.get(symlink.deref()) {
                    line = line.annotations(annotations.clone());
                }
            }
        }
        Status::Ok(line)
    }
}

fn handle_event_name(name: Option<&OsStr>) -> Option<(PathBuf, PathBuf)> {
    if let Some(file) = name {
        let path = PathBuf::from("/var/log/containers/").join(file);
        if let Ok(real) = canonicalize(&path) {
            return Some((path, real));
        }
    }
    None
}

#[derive(Deserialize, Serialize, Debug)]
struct Pod {
    metadata: Metadata,
}

#[derive(Deserialize, Serialize, Debug)]
struct Metadata {
    name: String,
    namespace: String,
    labels: KeyValueMap,
    annotations: KeyValueMap,
}