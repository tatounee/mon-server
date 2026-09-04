use std::sync::LazyLock;

use bytes::Bytes;
use http::Response;
use thiserror::Error;

use crate::services::serde::serialize;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("partial request")]
    PartialRequest,
}

static ERROR_500: LazyLock<Bytes> = LazyLock::new(|| {
    let response = Response::builder().status(500).body("").unwrap();

    serialize(response).unwrap()
});
