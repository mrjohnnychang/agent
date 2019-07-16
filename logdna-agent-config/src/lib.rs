use std::convert::TryFrom;
use std::error::Error;
use std::time::Duration;

use flate2::Compression;
use globber::Pattern;
use regex::Regex;

use agent_core::http::request::{Encoding, RequestTemplate, Schema};
use lazy_static::lazy_static;

use crate::raw::Config as RawConfig;

mod raw;

lazy_static! {
    static ref DEFAULT_EXCLUDE: Vec<Pattern> = vec![
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

    static ref DEFAULT_INCLUDE: Vec<Pattern> = vec![
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

impl TryFrom<RawConfig> for Config {
    type Error = Box<Error>;

    fn try_from(raw: RawConfig) -> Result<Self, Self::Error> {
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
            include: Rules {
                glob: DEFAULT_INCLUDE.clone(),
                regex: vec![],
            },
            exclude: Rules {
                glob: DEFAULT_EXCLUDE.clone(),
                regex: vec![],
            },
        };

        if let Some(mut raw_log_config) = raw.log {
            log_config.dirs.append(&mut raw_log_config.dirs);

            if let Some(rules) = raw_log_config.include {
                for rule in rules.regex {
                    log_config.include.regex.push(Regex::new(&rule)?)
                }

                for rule in rules.glob {
                    log_config.include.glob.push(Pattern::new(&rule)?)
                }
            }

            if let Some(rules) = raw_log_config.exclude {
                for rule in rules.regex {
                    log_config.exclude.regex.push(Regex::new(&rule)?)
                }

                for rule in rules.glob {
                    log_config.exclude.glob.push(Pattern::new(&rule)?)
                }
            }
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

pub struct HttpConfig {
    pub template: RequestTemplate,
    pub timeout: Duration,
    pub body_size: u64,
}

pub struct LogConfig {
    pub dirs: Vec<String>,
    pub include: Rules,
    pub exclude: Rules,
}

pub struct Rules {
    pub glob: Vec<Pattern>,
    pub regex: Vec<Regex>,
}