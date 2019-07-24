use std::time::Duration;


use agent_core::http::request::{RequestTemplate};
use agent_fs::rule::{Rules};

mod env;
mod raw;

pub struct Config {
    pub http: HttpConfig,
    pub log: LogConfig,
}

pub struct HttpConfig {
    pub template: RequestTemplate,
    pub timeout: Duration,
    pub body_size: u64,
}

pub struct LogConfig {
    pub dirs: Vec<String>,
    pub rules: Rules,
}