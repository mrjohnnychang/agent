use std::{io, fmt};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ConfigError {
    MissingField(String),
    Io(io::Error),
    Serde(serde_yaml::Error),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unimplemented!()
    }
}