use std::{
    borrow::ToOwned,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

use reqwest::{Client, RequestBuilder};

use serde_json::{json, Map};

use dashmap::{mapref::one::Ref, DashMap, Entry};
use rquickjs::{async_with, AsyncContext, AsyncRuntime};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    cipher::Cipher,
    clients::{ClientConfig,ClientType},
    errors::Error,
    query::WebSearch,
    structs::{Video, VideoFormat},
    utils::between,
};

/// Url to the player javascript, which contains information required to decipher stream url
/// This url tends to change and should be re-fetched occasionally.
struct PlayerUrl {
    url: String,
    expire_at: Instant,
}

impl PlayerUrl {
    pub fn new() -> Self {
        PlayerUrl {
            url: String::new(),
            expire_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expire_at <= Instant::now()
    }

    pub fn set_url(&mut self, url: String) {
        self.url = url;
        // expire one day from now, arbitrary
        self.expire_at = Instant::now() + Duration::from_secs(86400);
    }
}

/// Configuration options for the Innertube instance.
#[derive(Debug)]
pub struct Config {
    /// Configs to use for requests, the given order is the same order used when querying the api.
    ///
    /// Defaults to Ios and Web, same as yt-dlp
    pub configs: Vec<ClientConfig>,
    /// [`reqwest::Client`] to use
    ///
    /// Defaults to creating a new instance
    pub http: Client,
    /// How many times to retry a request before skipping the config in use. Set to 0 to disable
    /// retries.
    ///
    /// Defaults to 3
    pub retry_limit: i8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            configs: vec![
                ClientConfig::new(ClientType::Ios),
                ClientConfig::new(ClientType::Web),
            ],
            http: Client::new(),
            retry_limit: 3,
        }
    }
}

/// Main structure used for sending requests using the Innertube api. Retries and caching is
/// handled by this structure so reusing the same instance for multiple requests is recommended.
/// Namely, ciphers, solutions to the ciphered stream urls, are cached as computing them is not
/// cheap, requiring the use of multiple source scans and a javascript evaluator.
///
/// Use an [`std::sync::Arc`] if threading is required.
pub struct Innertube {
    configs: Vec<ClientConfig>,
    web_config: ClientConfig,

    http: Client,
    retry_limit: i8,
    player_url: Arc<Mutex<PlayerUrl>>,
    cipher_cache: DashMap<String, Cipher>,

    js_runtime: AsyncRuntime,
}

impl Innertube {
    /// Creates a new Innertube instance using the given config
    ///
    /// # Errors
    ///
    /// An error is returned if the quickjs runtime fails to initialize.
    pub fn new(config: Config) -> Result<Self, Error> {
        let js_runtime = AsyncRuntime::new().map_err(|e| Error::Unexpected(e.to_string()))?;

        Ok(Innertube {
            http: config.http,
            configs: config.configs,
            retry_limit: config.retry_limit,

            player_url: Arc::new(Mutex::new(PlayerUrl::new())),
            cipher_cache: DashMap::new(),
            web_config: ClientConfig::new(ClientType::Web),

            js_runtime,
        })
    }

    /// Deciphers a [`crate::structs::VideoFormat`] stream url. For some configs, Innertube responds
    /// with a ciphered url. It is impossible to send a request there so we have to decipher it.
    /// See [`crate::ciphers::Cipher`] for more information.
    ///
    /// # Errors
    ///
    /// This may fail if network requests or deserialization fails.
    /// Errors can also arise if the url can not be deciphered.
    /// An error from rquickjs is unexpected and should not happen.
    pub async fn decipher_format(&self, format: &VideoFormat) -> Result<String, Error> {
        let player_url = self.get_player_url().await?;
        let pair = self.get_cipher_pair(&player_url).await?;

        let context = AsyncContext::full(&self.js_runtime)
            .await
            .map_err(|e| Error::Unexpected(e.to_string()))?;

        async_with!(context => |ctx| {
            pair.value().apply(ctx, format)
        })
        .await
    }

    /// Queries the Innertube 'player' endpoint for information about a video.
    /// If a request fails, it is retried up to [`Self::retry_limit`] times.
    /// And if the request still fails, the next [`crate::clients::ClientConfig`] is used.
    /// Use [`decipher_format()`] on a [`VideoFormat`] in the [`Video`] is stream url is needed.
    ///
    /// # Errors
    ///
    /// This may fail if network requests or deserialization fails. If no video info can be found,
    /// an error is returned.
    pub async fn info(&self, video: &str) -> Result<Video, Error> {
        let video = get_video_id(video).ok_or(Error::NotYoutubeUrl(video.to_owned()))?;

        for config in &self.configs {
            let mut data = Map::new();
            data.insert("videoId".to_owned(), video.into());
            data.insert("context".to_owned(), config.context_json());
            data.insert("contentCheckOk".to_owned(), true.into());
            data.insert("racyCheckOk".to_owned(), true.into());

            if config.requires_player() {
                let player_url = self.get_player_url().await?;
                // Nothing we can do besides using the next config
                if player_url.is_empty() {
                    continue;
                }

                // Same here, timestamp is required otherwise we might get an unrelated response
                let pair = self.get_cipher_pair(&player_url).await?;
                if let Some(timestamp) = pair.value().timestamp() {
                    data.insert(
                        "playbackContext".to_owned(),
                        json!({
                            "contentPlaybackContext": {
                                "signatureTimestamp": timestamp,
                            }
                        }),
                    );
                } else {
                    continue;
                }
            }

            // TODO: also retry on http error?
            let data = data.into();
            for _attempt in 0..=self.retry_limit {
                let res = self
                    .build_request("player", config, &data)
                    .send()
                    .await?
                    .json::<Video>()
                    .await?;

                if !video_invalid(&res) {
                    return Ok(res);
                }
            }
        }
        Err(Error::VideoInfo)
    }

    /// Queries the Innertube 'search' endpoint for search results, returning a list of video ids.
    /// This request is not guarded with methods like the one above so methods like multiple
    /// clients or retries are not required.
    ///
    /// # Errors
    ///
    /// This may fail if network requests or deserialization fails.
    pub async fn search(&self, query: &str) -> Result<Vec<String>, Error> {
        // Params are to indicate videos only
        let data = json!({
            "query": query,
            "context": self.web_config.context_json(),
            "params": "EgIQAfABAQ==",
        });

        Ok(self
            .build_request("search", &self.web_config, &data)
            .send()
            .await?
            .json::<WebSearch>()
            .await?
            .queries())
    }

    /// Returns the cipher key for the given player url, creating one if it does not exist.
    async fn get_cipher_pair(&self, player_url: &str) -> Result<Ref<String, Cipher>, Error> {
        match self.cipher_cache.entry(player_url.to_string()) {
            Entry::Vacant(entry) => {
                let player_js = self.http.get(player_url).send().await?.text().await?;
                Ok(entry.insert(Cipher::new(&player_js)).downgrade())
            }
            Entry::Occupied(entry) => Ok(entry.into_ref().downgrade()),
        }
    }

    /// Gets the currently cached player js url, making a request to fetch it if it is expired.
    async fn get_player_url(&self) -> Result<String, Error> {
        let mut player_url = self.player_url.lock().await;
        if player_url.is_expired() {
            let res = self
                .http
                .get("https://www.youtube.com/embed/")
                .send()
                .await?
                .text()
                .await?;

            let url = between(&res, "\"jsUrl\":\"", "\"");
            let url = if url.starts_with("//") {
                "https:".to_owned() + url
            } else if url.starts_with('/') {
                "https://www.youtube.com".to_owned() + url
            } else {
                url.to_owned()
            };

            // Set it even if url is empty, handling the case later. This is to set the expiration
            // so the client does not keep sending requests until they get ratelimited. If it
            // fails once, it is not going to succeed anytime soon.
            player_url.set_url(url);
        }

        Ok(player_url.url.clone())
    }

    /// Builds a request to be sent to the Innertube api
    fn build_request(
        &self,
        endpoint: &str,
        config: &ClientConfig,
        data: &serde_json::Value,
    ) -> RequestBuilder {
        let url = format!(r"https:\\{}/youtubei/v1/{}", config.hostname(), endpoint);
        self.http
            .post(url)
            .headers(config.headers())
            .query(&[("key", config.api_key()), ("prettyPrint", "false")])
            .json(data)
    }
}

/// Attmpts to get the video id from a YouTube url
fn get_video_id(url: &str) -> Option<&str> {
    // from: https://stackoverflow.com/questions/5830387/how-do-i-find-all-youtube-video-ids-in-a-string-using-a-regex?noredirect=1&lq=1
    // this is a little lax however, but should suffice for most cases
    const YOUTUBE_URL: &str = r"(?xs)
        (?:https?:)?//              # protocol
        (?:[[:alnum:]-]+\.)?        # optional subdomain
        (?:                         # hostname
         youtu\.be/
         | youtube(?:-nocookie)?
         \.com
         \S*?                       # whatever until we find the id
         [^[:word:]\s-]             #
        )
        ([[:word:]w-]{11})          # id
    ";
    static PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(YOUTUBE_URL).unwrap());

    if let Some(cap) = PATTERN.captures(url) {
        return Some(cap.get(1).unwrap().as_str());
    } else if url.len() == 11
        && url
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Some(url);
    }
    None
}

/// Checks if a video is invalid. A video is marked invalid if a set of tokens, 51217102 or
/// 51217476 exist in the video, in which case any request results in a 403 error.
fn video_invalid(video: &Video) -> bool {
    // TODO: do both tokens have to exist?
    video
        .response_context
        .service_tracking_params
        .iter()
        .find(|service| service.service == "GFEEDBACK")
        .and_then(|service| {
            service
                .params
                .iter()
                .find(|param| param.key == "e")
                .map(|param| {
                    param
                        .value
                        .split(',')
                        .any(|x| x == "51217102" || x == "51217476")
                })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_video_id() {
        let strings = [
            "http://youtu.be/NLqAF9hrVbY",
            "http://www.youtube.com/embed/NLqAF9hrVbY",
            "http://www.youtube.com/v/NLqAF9hrVbY?fs=1&hl=en_US",
            "http://www.youtube.com/watch?v=NLqAF9hrVbY",
        ];
        for str in strings {
            assert_eq!(get_video_id(str), Some("NLqAF9hrVbY"));
        }
    }

    #[test]
    fn test_get_video_id_id_only() {
        let strings = ["NLqAF9hrVbY", "IB3lcPjvWLA", "BaW_jenozKc", "a9LDPn-MO4I"];
        for str in strings {
            assert_eq!(get_video_id(str), Some(str));
        }
    }

    #[test]
    fn test_get_video_id_bad() {
        let strings = [
            "NLqAF9hrVbYAFNE",
            "IB3lcPjvW",
            "IB3lcPjv!",
            "https://example.com/",
            "http://youtu.be/v=NLqAF9hrVbY",
            "http://www.youtube.com/     NLqAF9hrVbY",
        ];
        for str in strings {
            assert_eq!(get_video_id(str), None);
        }
    }
}
