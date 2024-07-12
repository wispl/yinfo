use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unable to send http request")]
    Request(#[from] reqwest::Error),
    #[error("unable to find player js url")]
    PlayerJS,
    #[error("unable to decipher signature: {0}")]
    Cipher(String),
    #[error("unable to extract nfunc")]
    NFuncExtraction,
    #[error("Could not execute javascript because of exception: {0}")]
    JSExecution(String),
    #[error("encounter enchanced except when executing javascript")]
    JSEnhancedExcept,
    #[error("unable to parse mime: expected '{0}', found '{1}'")]
    MimeParse(&'static str, String),
    #[error("{0} is missing from {1}")]
    MissingField(&'static str, &'static str),
    #[error("failed to parse url {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("unexpected error: {0}")]
    Unexpected(String),
}
