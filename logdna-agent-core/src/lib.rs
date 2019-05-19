pub use logdna_client;

pub mod http {
    pub use logdna_client::*;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
