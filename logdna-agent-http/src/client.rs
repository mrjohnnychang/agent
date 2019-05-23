use tokio::runtime::Runtime;

use agent_core::http::client::Client as HttpClient;
use agent_core::http::request::RequestTemplate;

pub struct Client {
    inner: HttpClient,
    runtime: Runtime,
}

impl Client {
    pub fn new(template: RequestTemplate) -> Self {
        let mut runtime = Runtime::new().expect("Runtime::new()");
        Self {
            inner: HttpClient::new(template, &mut runtime),
            runtime,
        }
    }
}