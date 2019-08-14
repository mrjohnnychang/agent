use std::convert::TryFrom;
use std::ffi::CString;
use std::path::PathBuf;
use std::time::Duration;

use flate2::Compression;

use http::types::request::{Encoding, RequestTemplate, Schema};
use fs::rule::{GlobRule, RegexRule, Rules};

use crate::error::ConfigError;
use crate::raw::Config as RawConfig;

mod env;
mod error;
mod raw;

#[derive(Debug)]
pub struct Config {
    pub http: HttpConfig,
    pub log: LogConfig,
}

#[derive(Debug)]
pub struct HttpConfig {
    pub template: RequestTemplate,
    pub timeout: Duration,
    pub body_size: u64,
}

#[derive(Debug)]
pub struct LogConfig {
    pub dirs: Vec<PathBuf>,
    pub rules: Rules,
}

impl TryFrom<RawConfig> for Config {
    type Error = ConfigError;

    fn try_from(raw: RawConfig) -> Result<Self, Self::Error> {
        let mut template_builder = RequestTemplate::builder();

        template_builder.api_key(
            raw.http.ingestion_key
                .ok_or(ConfigError::MissingField("http.ingestion_key"))?
        );

        let use_ssl = raw.http.use_ssl
            .ok_or(ConfigError::MissingField("http.use_ssl"))?;
        match use_ssl {
            true => template_builder.schema(Schema::Https),
            false => template_builder.schema(Schema::Http),
        };

        let use_compression = raw.http.use_compression
            .ok_or(ConfigError::MissingField("http.use_compression"))?;
        let gzip_level = raw.http.gzip_level
            .ok_or(ConfigError::MissingField("http.gzip_level"))?;
        match use_compression {
            true => template_builder.encoding(Encoding::GzipJson(Compression::new(gzip_level))),
            false => template_builder.encoding(Encoding::Json),
        };

        template_builder.host(
            raw.http.host
                .ok_or(ConfigError::MissingField("http.host"))?
        );

        template_builder.endpoint(
            raw.http.endpoint
                .ok_or(ConfigError::MissingField("http.endpoint"))?
        );

        template_builder.params(raw.http.params
            .ok_or(ConfigError::MissingField("http.params"))?);

        let http = HttpConfig {
            template: template_builder.build()?,
            timeout: Duration::from_secs(
                raw.http.timeout.
                    ok_or(ConfigError::MissingField("http.timeout"))?
            ),
            body_size: raw.http.body_size.
                ok_or(ConfigError::MissingField("http.body_size"))?,
        };

        let mut log = LogConfig {
            dirs: raw.log.dirs
                .into_iter()
                .map(|s| PathBuf::from(s))
                .collect(),
            rules: Rules::new(),
        };

        if let Some(rules) = raw.log.include {
            for glob in rules.glob {
                log.rules.add_inclusion(GlobRule::new(&*glob)?)
            }

            for regex in rules.regex {
                log.rules.add_inclusion(RegexRule::new(&*regex)?)
            }
        }

        if let Some(rules) = raw.log.exclude {
            for glob in rules.glob {
                log.rules.add_exclusion(GlobRule::new(&*glob)?)
            }

            for regex in rules.regex {
                log.rules.add_exclusion(RegexRule::new(&*regex)?)
            }
        }

        Ok(Config {
            http,
            log,
        })
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

    #[test]
    fn test_raw_to_typed() {
        let raw = RawConfig::default();
        assert!(Config::try_from(raw).is_err());
        let mut raw = RawConfig::default();
        raw.http.ingestion_key = Some("emptyingestionkey".to_string());
        assert!(Config::try_from(raw).is_ok());
    }

}