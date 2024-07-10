use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

use reqwest::{
    RequestBuilder,
    header::HeaderValue,
    Client,
};

use serde_json::json;

use rquickjs::{
    AsyncRuntime,
    AsyncContext,
    context::intrinsic::{Eval, Date},
};
use dashmap::{
    DashMap,
    Entry,
    mapref::one::Ref,
};

use crate::{
    clients,
    errors::Error,
    structs::{Video, VideoFormat},
    cipher::Cipher,
    query::WebSearch,
    utils::between,
};

// Javascript player URL, this contains the code for ciphers used to decipher signatures
pub struct PlayerUrl {
    url: String,
    expire_at: Instant,
}

impl PlayerUrl {
    pub fn new() -> Self {
        PlayerUrl {
            url: "".to_owned(),
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
    client: clients::ClientConfig,
    reqwest: Client,
    js_runtime: AsyncRuntime,

    player_url: Arc<Mutex<PlayerUrl>>,
    cipher_cache: DashMap<String, Cipher>
}

impl Innertube {
    pub fn new(reqwest: Client, client: clients::ClientConfig) -> Self {
        Innertube {
            reqwest,
            client,
            js_runtime: AsyncRuntime::new().unwrap(),
            player_url: Arc::new(Mutex::new(PlayerUrl::new())),
            cipher_cache: DashMap::new(),
        }
    }

    pub async fn decipher_format(&self, format: &VideoFormat) -> Result<String, Error> {
        let player_url = self.get_player_url().await?;
        let pair = self.get_cipher_pair(&player_url).await?;

        let context = AsyncContext::custom::<(Eval, Date)>(&self.js_runtime).await.unwrap();
        pair.value().apply(&context, format).await
    }

    pub async fn info(&self, video: &str) -> Result<Video, Error> {
        let player_url = self.get_player_url().await?;
        let pair = self.get_cipher_pair(&player_url).await?;
        let cipher = pair.value();

        // TODO: drop cipher after this
        let timestamp = json!({ "signatureTimestamp": cipher.timestamp() });

        let data = json!({
            "videoId": video,
            "context": self.client.context_json(),
            "playbackContext": json!({ "contentPlaybackContext": timestamp }),
            "contentCheckOk": true,
            "racyCheckOk": true,
        });


        self.build_request("player", data)
            .send()
            .await?
            .json::<Video>()
            .await
            .map_err(Error::Request)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<String>, Error> {
        let data = json!({
            "query": query,
            "context": self.client.context_json(),
            "params": "EgIQAfABAQ==",
        });

        Ok(self.build_request("search", data)
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
                let player_js = self.reqwest.get(player_url)
                    .send()
                    .await?
                    .text()
                    .await?;
                Ok(entry.insert(Cipher::new(&player_js)).downgrade())
            }
            Entry::Occupied(entry) => Ok(entry.into_ref().downgrade())
        }
    }

    async fn get_player_url(&self) -> Result<String, Error>{
        let mut player_url = self.player_url.lock().await;
        if player_url.is_expired() {
            let res = self.reqwest.get("https://www.youtube.com/embed/")
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

    fn build_request(&self, endpoint: &str, data: serde_json::Value) -> RequestBuilder {
        let url = format!(r"https:\\{}/youtubei/v1/{}", self.client.hostname(), endpoint);
        let mut headers = self.client.headers();
        let origin = format!(r"https:\\{}", self.client.hostname());
        headers.insert("Origin", HeaderValue::from_str(&origin).unwrap());
        self.reqwest.post(url)
            .headers(self.client.headers())
            .query(&[("key", self.client.api_key()), ("prettyPrint", "false")])
            .json(&data)
    }
}
