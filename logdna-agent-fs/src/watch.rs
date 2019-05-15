use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use inotify::{Event as InotifyEvent, EventMask, Inotify, WatchDescriptor};

use crate::Event;
use crate::macros;
use crate::rule::{Rule, Rules};

//todo provide examples and some extra tid bits around operational behavior
/// Used to watch the filesystem for [Events](../enum.Event.html)
///
/// Also has support for exclusion and inclusion rules to narrow the scope of watched files/directories
pub struct Watcher {
    // An instance of inotify
    inotify: Inotify,
    // A mapping of watch descriptors to paths
    // This is required because inotify operates on a watch list which (a list of i64s)
    // This provides a mapping of those ids to the corresponding paths
    // The invariant that is relied on here is that is mapping is always correct
    // The main mechanism for breaking this invariant is overflowing the kernel queue (Q_OVERFLOW)
    watch_descriptors: HashMap<WatchDescriptor, PathBuf>,
    // A list of inclusion and exclusion rules
    rules: Rules,
    // The list of dirs to watch on startup, e.g /var/log/
    // These dirs will be watched recursively
    // So if /var/log/ is in this list, /var/log/httpd/ is redundant
    initial_dirs: Vec<PathBuf>,
    // A duration that the event loop will wait before polling again
    // Effectively a dumb rate limit, in the case the sender is unbounded
    loop_interval: Duration,
}

impl Watcher {
    /// Creates an instance of WatchBuilder
    pub fn builder() -> WatchBuilder {
        WatchBuilder {
            initial_dirs: Vec::new(),
            loop_interval: Duration::from_millis(250),
            rules: Rules::new(),
        }
    }
    /// Runs the main logic loop of the watcher, consuming itself because run can only be called once
    ///
    /// The sender is the where events are streamed too, this should be an unbounded sender
    /// to prevent kernel over flow. However, being unbounded isn't a hard requirement.
    pub fn run(mut self, sender: Sender<Event>) {
        let mut buf = [0u8; 4096];
        loop {
            let events = match self.inotify.read_events_blocking(&mut buf) {
                Ok(events) => events,
                Err(e) => {
                    error!("error reading from inotify fd: {}", e);
                    continue;
                }
            };

            for event in events {
                self.process(event, &sender);
            }
        }
    }

    fn process(&mut self, event: InotifyEvent<&OsStr>, sender: &Sender<Event>) {
        if event.mask.contains(EventMask::CREATE) {}

        if event.mask.contains(EventMask::MODIFY) {}

        if event.mask.contains(EventMask::DELETE_SELF) {}
    }
}

/// Creates an instance of a Watcher
pub struct WatchBuilder {
    initial_dirs: Vec<PathBuf>,
    loop_interval: Duration,
    rules: Rules,

}

impl WatchBuilder {
    /// Add a dir to the list of initial dirs
    pub fn add<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.initial_dirs.push(path.into());
        self
    }
    /// Add a multiple dirs to the list of initial dirs
    pub fn add_all<T: AsRef<[PathBuf]>>(mut self, path: T) -> Self {
        self.initial_dirs.extend_from_slice(path.as_ref());
        self
    }
    /// Sets the loop interval
    pub fn loop_interval<T: Into<Duration>>(mut self, duration: T) -> Self {
        self.loop_interval = duration.into();
        self
    }
    /// Adds an inclusion rule
    pub fn include<T: Rule + Send + 'static>(mut self, rule: T) -> Self {
        self.rules.add_inclusion(rule);
        self
    }
    /// Adds an exclusion rule
    pub fn exclude<T: Rule + Send + 'static>(mut self, rule: T) -> Self {
        self.rules.add_exclusion(rule);
        self
    }
    /// Appends all rules from another instance of rules
    pub fn append_all<T: Into<Rules>>(mut self, rules: T) -> Self {
        self.rules.add_all(rules);
        self
    }
    /// Consumes the builder and produces an instance of the watcher
    pub fn build(self) -> Result<Watcher, io::Error> {
        Ok(Watcher {
            inotify: Inotify::init()?,
            watch_descriptors: HashMap::new(),
            rules: self.rules,
            initial_dirs: self.initial_dirs,
            loop_interval: self.loop_interval,
        })
    }
}