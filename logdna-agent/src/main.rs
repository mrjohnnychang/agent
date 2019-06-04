use std::env;
use std::thread::spawn;

use agent_core::http::params::Params;
use agent_core::http::request::RequestTemplate;
use agent_fs::rule::{GlobRule, RegexRule};
use agent_fs::tail::Tailer;
use agent_fs::watch::Watcher;
use agent_http::client::Client;

fn main() {
    env_logger::init();

    let watcher = Watcher::builder()
        .add("/var/log/")
        .include(GlobRule::new("*.log").unwrap())
        .include(RegexRule::new(r#"/.+/[^.]*$"#).unwrap())
        .build().unwrap();

    let tailer = Tailer::new();
    let tailer_sender = tailer.sender();

    let template = RequestTemplate::builder()
        .params(Params::builder().hostname("connor-pc").build().unwrap())
        .api_key(env::var("API_KEY").expect("api key missing"))
        .build().unwrap();

    let client = Client::new(template);
    let client_sender = client.sender();

    spawn(move || tailer.run(client_sender));
    spawn(move || watcher.run(tailer_sender));
    client.run()
}