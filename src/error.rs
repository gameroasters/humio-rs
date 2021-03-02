#![allow(clippy::pub_enum_variant_names)]
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("serde_json error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("http error: {0}")]
    HttpError(#[from] hyper::http::Error),

    #[error("hyper error: {0}")]
    HyperError(#[from] hyper::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
