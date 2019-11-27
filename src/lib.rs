pub mod advice;
pub mod client;
pub mod config;
pub mod error;
pub mod request;
pub mod response;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
