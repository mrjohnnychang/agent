use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

use agent_core::http::request::RequestTemplate;
use agent_fs::rule::Rules;

use crate::error::ConfigError;

mod env;
mod error;
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

impl Config {
    pub fn parse() -> Result<Self, ConfigError> {
        let mut content = String::new();
        File::open(path)?.read_to_string(&mut content)?;
        let config = serde_yaml::from_str(&content)?;
        config.into()
    }
}

pub fn get_hostname() -> Option<String> {
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
}