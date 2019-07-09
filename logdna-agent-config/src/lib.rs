use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    domain: Option<String>,
    endpoint: Option<String>,
    inclusion_rules: Option<Rules>,
    exclusion_rules: Option<Rules>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Rules {
    glob: Vec<String>,
    regex: Vec<String>,
}