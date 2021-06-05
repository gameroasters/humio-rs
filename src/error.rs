use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("http error: {0}")]
    Http(#[from] hyper::http::Error),

    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
