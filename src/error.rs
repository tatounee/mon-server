use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("partial request")]
    PartialRequest,
}
