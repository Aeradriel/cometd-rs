#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(msg: &str) -> Error {
        log::error!("{}", msg);
        Error {
            message: msg.to_owned(),
        }
    }
}
