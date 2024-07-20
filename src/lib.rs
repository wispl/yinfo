//! # yinfo
//!
//! This crates provides simple, and with best effort, robust, access to YouTube's hidden Innertube
//! api, which allows for querying information about searches, videos, and playlists.
//!
//! In short the crate uses a list of clients to send requests to the endpoint, skipping and
//! retrying clients as needed.
//!
//! ## Basic Usage
//!
//! The main interface is through [`Innertube`], which provides methods methods for
//! searching and getting videos.
//!
//! For example, to fetch information about a video
//!
//! ```no_run
//! # use yinfo::{Innertube, Config};
//! # async fn run() -> Result<(), Error> {
//! let innertube = Innertube::new(Config::default())?;
//! let video_info = innertube.info("https://www.youtube.com/watch?v=5C_HPTJg5ek").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Or to fetch search results as a list of video ids
//!
//! ```no_run
//! # use yinfo::{Innertube, Config, Error};
//! # async fn run() -> Result<(), Error> {
//! let innertube = Innertube::new(Config::default())?;
//! let results = innertube.search("how to learn rust").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Handling Video Data
//!
//! [`Innertube::info`] returns a [`Video`] structs with public fields. Some convenient methods
//! are exposed: `best_audio()`, `best_video()`, `all_formats()` to name a
//! few.
//!
//! If you decide to stream or download the format, you must call [`Innertube::decipher_format()`]
//! on the desired format, as formats are not automatically deciphered.
//!
//! ```no_run
//! # use yinfo::{Innertube, Config};
//! # async fn run() -> Result<(), Error> {
//! let innertube = Innertube::new(Config::default())?;
//! let video_info = innertube.info("https://www.youtube.com/watch?v=5C_HPTJg5ek").await?;
//! let format = video_info.best_audio()?;
//! let url = innertube.decipher_format(&format)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! You may wish to configure [`Innertube`] to improve robustness or if you simply want to clamp
//! down on the amount of requests.
//!
//! ```no_run
//! # use yinfo::{Innertube, Config, ClientConfig, ClientType, Error};
//! # fn run() -> Result<(), Error> {
//! let config = Config {
//!     http: reqwest::Client::new(),
//!     retry_limit: 1,
//!     configs: vec![
//!         ClientConfig::new(ClientType::Web),
//!         ClientConfig::new(ClientType::Ios),
//!         ClientConfig::new(ClientType::Android),
//!     ],
//! };
//! # Ok(())
//! # }
//! ```
//!
//! The Innertube api tends to change, so having fallback clients is reasonable, though at the cost
//! of potentially more requests and maybe even being ratelimited.
//!
//! Not all clients are the same, see [`ClientType`] for more information.
pub mod cipher;
pub mod clients;
pub mod errors;
pub mod innertube;
pub mod mime;
mod query;
pub mod structs;
mod utils;

pub use {
    clients::{ClientConfig, ClientType},
    errors::Error,
    innertube::{Config, Innertube},
    mime::{Acodec, Format, Mime, Vcodec},
    structs::{Video, VideoFormat},
};
