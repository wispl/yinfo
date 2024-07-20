use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unable to send http request")]
    Request(#[from] reqwest::Error),
    #[error("cipher error: {0}")]
    Cipher(String),
    #[error("Could not execute javascript because of exception: {0}")]
    JSExecution(String),
    #[error("encounter enchanced except when executing javascript")]
    JSEnhancedExcept,
    #[error("unable to parse mime: expected '{0}', found '{1}'")]
    MimeParse(&'static str, String),
    #[error("failed to parse url {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("unexpected error: {0}")]
    Unexpected(String),
    #[error("{0} is not a valid YouTube url")]
    NotYoutubeUrl(String),
    #[error("failed to find any info for video")]
    VideoInfo,
}
