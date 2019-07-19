use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::time::Duration;

use flate2::Compression;

use agent_core::http::request::{Encoding, RequestTemplate, Schema};
use agent_fs::rule::{GlobRule, RegexRule, Rules};
use lazy_static::lazy_static;

use crate::raw::Config as RawConfig;

mod raw;

lazy_static! {
    static ref DEFAULT_EXCLUDE: Vec<GlobRule> = vec![
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
    ];

    static ref DEFAULT_INCLUDE: Vec<GlobRule> = vec![
        "*.log".parse().unwrap(),
        "!(*.*)".parse().unwrap(),
    ];

    static ref DEFAULT_COMPRESSION: Compression = Compression::new(2);

    static ref DEFAULT_LOG_DIR: String = "/var/log/".to_string();

    static ref DEFAULT_TIMEOUT: Duration = Duration::from_millis(10_000);

    static ref DEFAULT_BODY_SIZE: u64 = 2 * 1024 * 1024;
}

pub struct Config {
    pub http: HttpConfig,
    pub log: LogConfig,
}

impl Config {
    //FIXME: replace Box<Error> with a proper error type
    pub fn new(raw: RawConfig) -> Result<Self, Box<Error>> {
        let mut template_builder = RequestTemplate::builder();

        template_builder.api_key(raw.http.ingestion_key);

        if let Some(host) = raw.http.host {
            template_builder.host(host);
        }

        if let Some(endpoint) = raw.http.endpoint {
            template_builder.endpoint(endpoint);
        }

        if let Some(https) = raw.http.https {
            if https {
                template_builder.schema(Schema::Https);
            } else {
                template_builder.schema(Schema::Http);
            }
        }

        match (raw.http.compress, raw.http.compression_level) {
            (Some(compress), Some(level)) => {
                if compress {
                    template_builder.encoding(Encoding::GzipJson(Compression::new(level)));
                } else {
                    template_builder.encoding(Encoding::Json);
                }
            }
            (Some(compress), None) => {
                if compress {
                    template_builder.encoding(Encoding::GzipJson(*DEFAULT_COMPRESSION));
                } else {
                    template_builder.encoding(Encoding::Json);
                }
            }
            (None, Some(level)) => {
                template_builder.encoding(Encoding::GzipJson(Compression::new(level)));
            }
            (None, None) => {}
        }

        if let Some(params) = raw.http.params {
            template_builder.params(params);
        }

        let mut log_config = LogConfig {
            dirs: vec![DEFAULT_LOG_DIR.clone()],
            rules: Rules::new(),
        };

        if let Some(mut raw_log_config) = raw.log {
            log_config.dirs.append(&mut raw_log_config.dirs);

            if let Some(rules) = raw_log_config.include {
                for rule in rules.regex {
                    log_config.rules.add_inclusion(RegexRule::new(&*rule)?)
                }

                for rule in rules.glob {
                    log_config.rules.add_inclusion(GlobRule::new(&*rule)?)
                }
            }

            if let Some(rules) = raw_log_config.exclude {
                for rule in rules.regex {
                    log_config.rules.add_exclusion(RegexRule::new(&*rule)?)
                }

                for rule in rules.glob {
                    log_config.rules.add_exclusion(GlobRule::new(&*rule)?)
                }
            }
        }

        //FIXME: deprecate then remove env var
        if let Ok(host) = env::var("LDLOGHOST") {
            template_builder.host(host);
        }

        //FIXME: deprecate then remove env var
        if let Ok(endpoint) = env::var("LDLOGPATH") {
            template_builder.endpoint(endpoint);
        }

        //FIXME: deprecate then remove env var
        if let Ok(rules) = env::var("LOGDNA_EXCLUDE") {
            rules.split_terminator(",")
                .into_iter()
                .filter(|r| !r.is_empty())
                .filter_map(|r| GlobRule::new(r).ok())
                .for_each(|r| log_config.rules.add_exclusion(r))
        }

        //FIXME: deprecate then remove env var
        if let Ok(rules) = env::var("LOGDNA_EXCLUDE_REGEX") {
            rules.split_terminator(",")
                .into_iter()
                .filter(|r| !r.is_empty())
                .filter_map(|r| RegexRule::new(r).ok())
                .for_each(|r| log_config.rules.add_exclusion(r))
        }

        //FIXME: deprecate then remove env var
        if let Ok(rules) = env::var("LOGDNA_INCLUDE") {
            rules.split_terminator(",")
                .into_iter()
                .filter(|r| !r.is_empty())
                .filter_map(|r| GlobRule::new(r).ok())
                .for_each(|r| log_config.rules.add_inclusion(r))
        }

        //FIXME: deprecate then remove env var
        if let Ok(rules) = env::var("LOGDNA_INCLUDE_REGEX") {
            rules.split_terminator(",")
                .into_iter()
                .filter(|r| !r.is_empty())
                .filter_map(|r| RegexRule::new(r).ok())
                .for_each(|r| log_config.rules.add_inclusion(r))
        }

        Ok(Config {
            http: HttpConfig {
                template: template_builder.build()?,
                timeout: raw.http.timeout
                    .map(|t| Duration::from_millis(t))
                    .unwrap_or(DEFAULT_TIMEOUT.clone()),
                body_size: raw.http.body_size.unwrap_or(*DEFAULT_BODY_SIZE),
            },
            log: log_config,
        })
    }
}

impl TryFrom<RawConfig> for Config {
    type Error = Box<Error>;

    fn try_from(raw: RawConfig) -> Result<Self, Self::Error> {
        Config::new(raw)
    }
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