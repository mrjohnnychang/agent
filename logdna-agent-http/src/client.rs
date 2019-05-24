use tokio::runtime::{Builder, Runtime};

use agent_core::http::client::Client as HttpClient;
use agent_core::http::request::RequestTemplate;

pub struct Client {
    inner: HttpClient,
    runtime: Runtime,
}

impl Client {
    pub fn new(template: RequestTemplate) -> Self {
        let mut runtime = Builder::new()
            .core_threads(1)
            .build()
            .expect("Runtime::new()");
        Self {
            inner: HttpClient::new(template, &mut runtime),
            runtime,
        }
    }
}