use serde::{Deserialize, Serialize};

use agent_core::http::params::Params;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub http: HttpConfig,
    pub log: Option<LogConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpConfig {
    pub host: Option<String>,
    pub endpoint: Option<String>,
    pub https: Option<bool>,
    pub timeout: Option<u64>,
    pub compress: Option<bool>,
    pub compression_level: Option<u32>,
    pub ingestion_key: String,
    pub params: Option<Params>,
    pub body_size: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogConfig {
    pub dirs: Vec<String>,
    pub include: Option<Rules>,
    pub exclude: Option<Rules>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Rules {
    pub glob: Vec<String>,
    pub regex: Vec<String>,
}
