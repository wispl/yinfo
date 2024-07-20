use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// Reqwest-related errors such as network or redirect based.
    #[error("unable to send http request")]
    Reqwest(#[from] reqwest::Error),

    /// Url parse errors.
    #[error("failed to parse url {0}")]
    UrlParse(#[from] url::ParseError),

    /// Cipher related errors.
    #[error("cipher error: {0}")]
    Cipher(String),

    /// Normal javascript execution errors from quickjs.
    #[error("Could not execute javascript because of exception: {0}")]
    JSExecution(String),

    /// YouTube nfunc code encountered an enhanced except.
    #[error("encounter enhanced except when executing javascript")]
    JSEnhancedExcept,

    /// Mime parse errors.
    #[error("unable to parse mime: expected '{0}', found '{1}'")]
    MimeParse(&'static str, String),

    /// Given url is not a YouTube url.
    #[error("{0} is not a valid YouTube url")]
    NotYoutubeUrl(String),

    /// Unable to find any information on video, it could be the Innertube api might have changed
    /// or your IP might be banned or ratelimited.
    #[error("failed to find any info for video")]
    VideoInfo,

    /// Totally unexpected errors, these should be rare and are usually from quickjs.
    #[error("unexpected error: {0}")]
    Unexpected(String),
}
