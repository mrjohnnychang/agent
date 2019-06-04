use std::thread::spawn;

use agent_fs::tail::Tailer;
use agent_fs::watch::Watcher;
use agent_http::client::Client;

fn main() {
    pretty_env_logger::init();
    let watcher = Watcher::builder()
        .add("/var/log/")
        .include(GlobRule::new("*.log").unwrap())
        .include(RegexRule::new(r#"/.+/[^.]*$"#).unwrap())
        .build().unwrap();
    let tailer = Tailer::new();
    let tailer = Tailer::new();
    let tailer_sender = tailer.sender();
    spawn(move || tailer.run(s));
    spawn(move || watcher.run(tailer_sender).run(s));
}