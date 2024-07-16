use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

use reqwest::{Client, RequestBuilder};

use serde_json::json;

use dashmap::{mapref::one::Ref, DashMap, Entry};
use rquickjs::{async_with, AsyncContext, AsyncRuntime};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    cipher::Cipher,
    clients,
    errors::Error,
    query::WebSearch,
    structs::{Video, VideoFormat},
    utils::between,
};

// Javascript player URL, this contains the code for ciphers used to decipher signatures
pub struct PlayerUrl {
    url: String,
    expire_at: Instant,
}

impl Default for PlayerUrl {
    fn default() -> Self {
        Self::new()
    }
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

pub struct Innertube {
    configs: Vec<clients::ClientConfig>,
    http: Client,
    js_runtime: AsyncRuntime,

    player_url: Arc<Mutex<PlayerUrl>>,
    cipher_cache: DashMap<String, Cipher>,
}

impl Innertube {
    /// # Errors
    ///
    /// This generally should not fail unless rquickjs fails to create a runtime,
    /// in which case out of memory is the case.
    pub fn new(http: Client, configs: Vec<clients::ClientConfig>) -> Result<Self, Error> {
        let js_runtime = AsyncRuntime::new().map_err(|e| Error::Unexpected(e.to_string()))?;

        Ok(Innertube {
            http,
            configs,
            js_runtime,
            player_url: Arc::new(Mutex::new(PlayerUrl::new())),
            cipher_cache: DashMap::new(),
        })
    }

    /// # Errors
    ///
    /// This may normally fail as a result of a network error or cipher fails to parse.
    /// An unexpected case is if rquickjs fails to create a context.
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

    /// # Errors
    ///
    /// This may fail as a result of a network error. Cipher and json errors are unexpected and
    /// indicates something in the library must be changed.
    pub async fn info(&self, video: &str) -> Result<Video, Error> {
        let video = get_video_id(video).ok_or(Error::NotYoutubeUrl(video.to_owned()))?;

        for config in &self.configs {
            let player_url = self.get_player_url().await?;
            let pair = self.get_cipher_pair(&player_url).await?;
            let cipher = pair.value();

            // TODO: drop cipher after this
            let timestamp = json!({ "signatureTimestamp": cipher.timestamp() });

            let data = json!({
                "videoId": video,
                "context": config.context_json(),
                "playbackContext": json!({ "contentPlaybackContext": timestamp }),
                "contentCheckOk": true,
                "racyCheckOk": true,
            });

            return self.build_request("player", config, &data)
                .send()
                .await?
                .json::<Video>()
                .await
                .map_err(Error::Request);
        }
        Err(Error::VideoInfo)
    }

    /// # Errors
    ///
    /// This may fail as a result of a network error. Cipher and json errors are unexpected and
    /// indicates something in the library must be changed.
    pub async fn search(&self, query: &str) -> Result<Vec<String>, Error> {
        // TODO: just use the first one for now, subject to change of course
        let config = self.configs.first().unwrap();
        let data = json!({
            "query": query,
            "context": config.context_json(),
            "params": "EgIQAfABAQ==",
        });

        Ok(self
            .build_request("search", config, &data)
            .send()
            .await?
            .json::<WebSearch>()
            .await?
            .queries())
    }

    // returns the cipher key and value pair Key is not needed but... avoids borrow issues here
    async fn get_cipher_pair(&self, player_url: &str) -> Result<Ref<String, Cipher>, Error> {
        // TODO: investigat deadlock?
        match self.cipher_cache.entry(player_url.to_string()) {
            Entry::Vacant(entry) => {
                let player_js = self.http.get(player_url).send().await?.text().await?;
                Ok(entry.insert(Cipher::new(&player_js)).downgrade())
            }
            Entry::Occupied(entry) => Ok(entry.into_ref().downgrade()),
        }
    }

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
            if url.is_empty() {
                return Err(Error::PlayerJS);
            }

            let url = if url.starts_with("//") {
                "https:".to_owned() + url
            } else if url.starts_with('/') {
                "https://www.youtube.com".to_owned() + url
            } else {
                url.to_owned()
            };

            player_url.set_url(url);
        }

        // TODO: cloning here is the easy out, check if we can return the mutex instead
        Ok(player_url.url.clone())
    }

    // TODO: implement
    // async fn get_ytcfg(&self) -> Result<(), Error> {
    //     static YTCFG: Lazy<Regex> = Lazy::new(||
    //         Regex::new(r"ytcfg\.set\((\{.+\})\);").unwrap()
    //     );
    //     let res = client.get("https://www.youtube.com")
    //         .send()
    //         .await?
    //         .text()
    //         .await?;
    //
    //     if let Some(cap) = YTCFG.captures(&res) {
    //         let json: serde_json::Value = serde_json::from_str(&cap[1]).unwrap();
    //         let client = &json["INNERTUBE_CONTEXT"]["client"];
    //         let client_version = &client["clientVersion"];
    //         let api_key = &json["INNERTUBE_API_KEY"];
    //
    //         let mut config = InnertubeClient::new(ClientType::Web);
    //
    //         if !client_version.is_null() {
    //             config.client.version = client_version.as_str().unwrap().to_owned();
    //         }
    //
    //         if !api_key.is_null() {
    //             config.api_key = client_version.as_str().unwrap().to_owned();
    //         }
    //
    //         Ok(config)
    //     } else {
    //         Err(Error::Cipher("failed to extract operations!".to_owned()))
    //     }
    // }

    fn build_request(
        &self,
        endpoint: &str,
        config: &clients::ClientConfig,
        data: &serde_json::Value
    ) -> RequestBuilder {
        let url = format!(r"https:\\{}/youtubei/v1/{}", config.hostname(), endpoint);
        self.http
            .post(url)
            .headers(config.headers())
            .query(&[("key", config.api_key()), ("prettyPrint", "false")])
            .json(data)
    }
}

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
