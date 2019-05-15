quick_error! {
    #[derive(Debug)]
    pub enum TailError {
        Io(err: std::io::Error) {
            from()
            display("I/O error: {}", err)
        }
    }
}
