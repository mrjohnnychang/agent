use std::any::Any;

pub use logdna_client;
use logdna_client::body::LineBuilder;

pub mod http {
    pub use logdna_client::*;
}

pub trait Middleware {
    fn process(&self, line: LineBuilder) -> Option<LineBuilder>;
}

pub trait EventListener {
    fn process(&self, event: Box<dyn Any>);
}