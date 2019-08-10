use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    // LOGDNA_CONFIG_FILE
    // deprecated: DEFAULT_CONF_FILE
    pub config_file: PathBuf,
    // LOGDNA_HOST
    // deprecated: LDLOGHOST
    pub host: Option<String>,
    // LOGDNA_ENDPOINT
    // deprecated: LDLOGPATH
    pub endpoint: Option<String>,
    // LOGDNA_INGESTION_KEY
    // deprecated: LOGDNA_AGENT_KEY
    pub ingestion_key: Option<String>,
    // LOGDNA_USE_SSL
    // deprecated: LDLOGSSL
    pub use_ssl: Option<bool>,
    // LOGDNA_USE_COMPRESSION
    // deprecated: COMPRESS
    pub use_compression: Option<bool>,
    // LOGDNA_GZIP_LEVEL
    // deprecated: GZIP_COMPRESS_LEVEL
    pub gzip_level: Option<u32>,
    // LOGDNA_HOSTNAME
    pub hostname: Option<String>,
    // LOGDNA_IP
    pub ip: Option<String>,
    // LOGDNA_TAGS
    pub tags: Option<Vec<String>>,
    // LOGDNA_MAC
    pub mac: Option<String>,
    // LOGDNA_LOG_DIRS
    // deprecated: LOG_DIRS
    pub log_dirs: Option<Vec<PathBuf>>,
    // LOGDNA_EXCLUSION_RULES
    // deprecated: LOGDNA_EXCLUDE
    pub exclusion_rules: Option<Vec<String>>,
    // LOGDNA_EXCLUSION_REGEX_RULES
    // deprecated: LOGDNA_EXCLUDE_REGEX
    pub exclusion_regex_rules: Option<Vec<String>>,
    // LOGDNA_INCLUSION_RULES
    // deprecated: LOGDNA_INCLUDE
    pub inclusion_rules: Option<Vec<String>>,
    // LOGDNA_INCLUSION_REGEX_RULES
    // deprecated: LOGDNA_INCLUDE_REGEX
    pub inclusion_regex_rules: Option<Vec<String>>,
}

impl Config {
    pub fn parse() -> Self {
        Self {
            config_file: parse_value(&["LOGDNA_CONFIG_FILE", "DEFAULT_CONF_FILE"])
                .unwrap_or_else(|| PathBuf::from("/etc/logdna/config.yaml")),
            host: parse_value(&["LOGDNA_HOST", "LDLOGHOST"]),
            endpoint: parse_value(&["LOGDNA_ENDPOINT", "LDLOGPATH"]),
            ingestion_key: parse_value(&["LOGDNA_INGESTION_KEY", "LOGDNA_AGENT_KEY"]),
            use_ssl: parse_value(&["LOGDNA_USE_SSL", "LDLOGSSL"]),
            use_compression: parse_value(&["LOGDNA_USE_COMPRESSION", "COMPRESS"]),
            gzip_level: parse_value(&["LOGDNA_GZIP_LEVEL", "GZIP_COMPRESS_LEVEL"]),
            hostname: parse_value(&["LOGDNA_HOSTNAME"]),
            ip: parse_value(&["LOGDNA_IP"]),
            tags: parse_list(&["LOGDNA_TAGS"]),
            mac: parse_value(&["LOGDNA_MAC"]),
            log_dirs: parse_list(&["LOGDNA_LOG_DIRS", "LOG_DIRS"]),
            exclusion_rules: parse_list(&["LOGDNA_EXCLUSION_RULES", "LOGDNA_EXCLUDE"]),
            exclusion_regex_rules: parse_list(&["LOGDNA_EXCLUSION_REGEX_RULES", "LOGDNA_EXCLUDE_REGEX"]),
            inclusion_rules: parse_list(&["LOGDNA_INCLUSION_RULES", "LOGDNA_INCLUDE"]),
            inclusion_regex_rules: parse_list(&["LOGDNA_INCLUSION_REGEX_RULES", "LOGDNA_INCLUDE_REGEX"]),
        }
    }
}

fn first_non_empty(envs: &[&str]) -> Option<String> {
    for env in envs {
        if let Ok(v) = env::var(env) {
            return Some(v);
        }
    }
    return None;
}

fn parse_list<T: FromStr>(envs: &[&str]) -> Option<Vec<T>> {
    Some(
        first_non_empty(envs)?
            .split_terminator(",")
            .filter_map(|s| T::from_str(s).ok())
            .collect()
    )
}

fn parse_value<T: FromStr>(envs: &[&str]) -> Option<T> {
    first_non_empty(envs).and_then(|s| T::from_str(&s).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_config() {
        macro_rules! env_test {
            ($envs:expr => $field:ident => $data:expr, $raw:expr) => {
                for env in $envs {
                    // reset env vars
                    for env in $envs {
                        env::remove_var(env);
                    }
                    env::set_var(env, $raw);
                    assert_eq!(Config::parse().$field, $data);
                }
            };
        }

        env_test!(
            &["LOGDNA_CONFIG_FILE", "DEFAULT_CONF_FILE"]
            => config_file
            => PathBuf::from("/test/path"), "/test/path"
        );

        env_test!(
            &["LOGDNA_HOST", "LDLOGHOST"]
            => host
            => Some("logs.localhost.com".to_string()), "logs.localhost.com"
        );

        env_test!(
            &["LOGDNA_ENDPOINT", "LDLOGPATH"]
            => endpoint
            => Some("/logs/test".to_string()), "/logs/test"
        );

        env_test!(
            &["LOGDNA_INGESTION_KEY", "LOGDNA_AGENT_KEY"]
            => ingestion_key
            => Some("supersecretkey".to_string()), "supersecretkey"
        );

        env_test!(
            &["LOGDNA_USE_SSL", "LDLOGSSL"]
            => use_ssl
            => Some(true), "true"
        );

        env_test!(
            &["LOGDNA_USE_COMPRESSION", "COMPRESS"]
            => use_compression
            => Some(true), "true"
        );

        env_test!(
            &["LOGDNA_GZIP_LEVEL", "GZIP_COMPRESS_LEVEL"]
            => gzip_level
            => Some(2), "2"
        );

        env_test!(
            &["LOGDNA_HOSTNAME"]
            => hostname
            => Some("unit-test".to_string()), "unit-test"
        );

        env_test!(
            &["LOGDNA_IP"]
            => ip
            => Some("127.0.0.1".to_string()), "127.0.0.1"
        );

        env_test!(
            &["LOGDNA_TAGS"]
            => tags
            => Some(vec!("prod".to_string(),"test".to_string(),"stage".to_string())), "prod,test,stage,"
        );

        env_test!(
            &["LOGDNA_MAC"]
            => mac
            => Some("AA:DD::EE::DD".to_string()), "AA:DD::EE::DD"
        );

        env_test!(
            & ["LOGDNA_LOG_DIRS", "LOG_DIRS"]
            => log_dirs
            => Some(vec!(PathBuf::from("/var/log/"),PathBuf::from("/tmp"),PathBuf::from("/var"))),
            "/var/log/,/tmp,/var,"
        );

        env_test!(
            &["LOGDNA_EXCLUSION_RULES", "LOGDNA_EXCLUDE"]
            => exclusion_rules
            => Some(vec!("*.log".to_string(),"!(*.*)".to_string(),"/var/log/**".to_string())), "*.log,!(*.*),/var/log/**,"
        );

        env_test!(
            &["LOGDNA_EXCLUSION_REGEX_RULES", "LOGDNA_EXCLUDE_REGEX"]
            => exclusion_regex_rules
            => Some(vec!(".+\\.log".to_string(),".*".to_string(),"\\w+\\d*".to_string())), ".+\\.log,.*,\\w+\\d*,"
        );

        env_test!(
            &["LOGDNA_INCLUSION_RULES", "LOGDNA_INCLUDE"]
            => inclusion_rules
            => Some(vec!("*.log".to_string(),"!(*.*)".to_string(),"/var/log/**".to_string())), "*.log,!(*.*),/var/log/**,"
        );

        env_test!(
            &["LOGDNA_INCLUSION_REGEX_RULES", "LOGDNA_INCLUDE_REGEX"]
            => inclusion_regex_rules
            => Some(vec!(".+\\.log".to_string(),".*".to_string(),"\\w+\\d*".to_string())), ".+\\.log,.*,\\w+\\d*,"
        );
    }
}