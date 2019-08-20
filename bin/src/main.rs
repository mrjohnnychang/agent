#[macro_use]
extern crate log;

use std::convert::TryFrom;
use std::thread::spawn;

use config::{env::Config as EnvConfig, raw::Config as RawConfig};
use config::Config;
use fs::tail::Tailer;
use fs::watch::Watcher;
use http::client::Client;
use http::retry::Retry;

fn main() {
    env_logger::init();

    let config = match Config::new() {
        Ok(v) => v,
        Err(e) => {
            error!("failed to load config: {}", e);
            warn!("falling back to default config!");
            match Config::try_from((EnvConfig::parse(), RawConfig::default())) {
                Ok(v) => v,
                Err(e) => {
                    error!("falling back to default failed: {}", e);
                    panic!()
                }
            }
        }
    };

    let watcher = Watcher::builder()
        .add_all(config.log.dirs)
        .append_all(config.log.rules)
        .build()
        .unwrap();

    let tailer = Tailer::new();
    let tailer_sender = tailer.sender();

    let mut client = Client::new(config.http.template);
    client.set_max_buffer_size(config.http.body_size);
    client.set_timeout(config.http.timeout);
    let (client_sender, client_retry_sender) = client.sender();

    let retry = Retry::new();
    let retry_sender = retry.sender();

    let tmp = client_sender.clone();
    spawn(move || tailer.run(tmp));
    spawn(move || retry.run(client_retry_sender));
    spawn(move || watcher.run(tailer_sender));
    client.run(retry_sender);
}