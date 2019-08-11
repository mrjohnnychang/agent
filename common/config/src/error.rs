use std::{fmt, io};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ConfigError {
    MissingField(&'static str),
    Io(io::Error),
    Serde(serde_yaml::Error),
    Template(http::types::error::TemplateError),
    Glob(globber::Error),
    Regex(regex::Error),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ConfigError::MissingField(field) => write!(f, "config error: {} is a required field", field),
            ConfigError::Io(e) => write!(f, "config error: {}", e),
            ConfigError::Serde(e) => write!(f, "config error: {}", e),
            ConfigError::Template(e) => write!(f, "config error: {}", e),
            ConfigError::Glob(e) => write!(f, "config error: {}", e),
            ConfigError::Regex(e) => write!(f, "config error: {}", e),
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(e: serde_yaml::Error) -> Self {
        ConfigError::Serde(e)
    }
}

impl From<http::types::error::TemplateError> for ConfigError {
    fn from(e: http::types::error::TemplateError) -> Self {
        ConfigError::Template(e)
    }
}

impl From<globber::Error> for ConfigError {
    fn from(e: globber::Error) -> Self {
        ConfigError::Glob(e)
    }
}

impl From<regex::Error> for ConfigError {
    fn from(e: regex::Error) -> Self {
        ConfigError::Regex(e)
    }
}