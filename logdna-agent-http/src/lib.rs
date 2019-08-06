#[macro_use]
extern crate log;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate crossbeam;

pub mod client;
pub mod retry;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
