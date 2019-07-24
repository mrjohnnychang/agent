use std::ffi::CString;

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
    pub ingestion_key: Option<String>,
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

impl Default for Config {
    fn default() -> Self {
        Config {
            http: HttpConfig::default(),
            log: Some(LogConfig::default()),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            host: Some("logs.logdna.com".to_string()),
            endpoint: Some("/logs/agent".to_string()),
            https: Some(true),
            timeout: Some(10_000),
            compress: Some(true),
            compression_level: Some(2),
            ingestion_key: None,
            params: Params::builder()
                .hostname(get_hostname().unwrap_or(String::new()))
                .build()
                .ok(),
            body_size: Some(2 * 1024 * 1024),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            dirs: vec!["/var/logs/".to_string()],
            include: Some(Rules {
                glob: vec![
                    "*.log".parse().unwrap(),
                    "!(*.*)".parse().unwrap(),
                ],
                regex: Vec::new(),
            }),
            exclude: Some(Rules {
                glob: vec![
                    "/var/log/wtmp".parse().unwrap(),
                    "/var/log/btmp".parse().unwrap(),
                    "/var/log/utmp".parse().unwrap(),
                    "/var/log/wtmpx".parse().unwrap(),
                    "/var/log/btmpx".parse().unwrap(),
                    "/var/log/utmpx".parse().unwrap(),
                    "/var/log/asl/**".parse().unwrap(),
                    "/var/log/sa/**".parse().unwrap(),
                    "/var/log/sar*".parse().unwrap(),
                    "/var/log/tallylog".parse().unwrap(),
                    "/var/log/fluentd-buffers/**/*".parse().unwrap(),
                ],
                regex: Vec::new(),
            }),
        }
    }
}


fn get_hostname() -> Option<String> {
    let name = CString::new(Vec::with_capacity(512)).ok()?.into_raw();
    if unsafe { libc::gethostname(name, 512) } == 0 {
        return unsafe { CString::from_raw(name) }.into_string().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hostname() {
        assert!(get_hostname().is_some());
    }

    #[test]
    fn test_default() {
        // test for panic at creation
        let config = Config::default();
        assert!(config.log.is_some());
        // make sure the config is serialise
        assert!(serde_yaml::to_string(&config).is_ok());
    }
}
