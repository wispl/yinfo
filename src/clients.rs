use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Map};

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/74.0.3729.157 Safari/537.36";

#[derive(Debug)]
pub enum ClientType {
    Web,
    Android,
    AndroidEmbedded,
    IOS,
}

#[derive(Debug)]
struct Client {
    name: &'static str,
    version: &'static str,
    user_agent: Option<&'static str>,
    sdk: Option<u8>,
}

#[derive(Debug)]
pub struct ClientConfig {
    client_type: ClientType,
    // name: &'static str,
    api_key: &'static str,
    context_client_name: &'static str,
    client: Client,
}

impl ClientConfig {
    pub fn context_json(&self) -> serde_json::Value {
        let mut context = Map::new();

        let mut client = Map::new();
        client.insert("clientName".to_owned(), self.client.name.into());
        client.insert("clientVersion".to_owned(), self.client.version.into());
        client.insert("hl".to_owned(), "en".to_owned().into());

        if let Some(sdk) = self.client.sdk {
            client.insert("androidSdkVersion".to_owned(), sdk.into());
        }

        if self.is_base() {
            client.insert("clientScreen".to_owned(), "EMBED".into());
            context.insert("thirdParty".to_owned(), json!({ "embedUrl": "https://www.youtube.com/" }));
        } else {
            context.insert("thirdParty".to_owned(), json!({ "embedUrl": "https://www.youtube.com/" }));
        }

        context.insert("client".to_owned(), client.into());
        json!(context)
    }

    pub fn is_base(&self) -> bool {
        matches!(self.client_type, ClientType::Web | ClientType::Android | ClientType::IOS)
    }

    pub fn hostname(&self) -> &str {
        "www.youtube.com"
    }

    pub fn api_key(&self) -> &str {
        self.api_key
    }

    pub fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("x-youtube-client-name", HeaderValue::from_static(self.context_client_name));
        headers.insert("x-youtube-client-version", HeaderValue::from_static(self.client.version));
        headers.insert(
            "user-agent",
            HeaderValue::from_static(self.client.user_agent.unwrap_or(DEFAULT_USER_AGENT))
        );

        headers
    }

    pub fn new(client_type: ClientType) -> Self {
        match client_type {
            ClientType::Web => ClientConfig {
                client_type,
                // name: "web",
                api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
                context_client_name: "1",
                client: Client {
                    name: "WEB",
                    version: "2.20220801.00.00",
                    user_agent: None,
                    sdk: None,
                }
            },
            ClientType::Android => ClientConfig {
                client_type,
                // name: "android",
                api_key: "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w",
                context_client_name: "3",
                client: Client {
                    name: "ANDROID",
                    version: "19.09.37",
                    sdk: Some(30),
                    user_agent: Some("com.google.android.youtube/19.09.37 (Linux; U; Android 11) gzip"),
                }
            },
            ClientType::AndroidEmbedded => ClientConfig {
                client_type,
                // name: "android_embedded",
                api_key: "AIzaSyCjc_pVEDi4qsv5MtC2dMXzpIaDoRFLsxw",
                context_client_name: "55",
                client: Client {
                    name: "ANDROID_EMBEDDED_PLAYER",
                    version: "19.09.37",
                    sdk: Some(30),
                    user_agent: Some("com.google.android.youtube/19.09.37 (Linux; U; Android 11) gzip"),
                }
            },
            ClientType::IOS => ClientConfig {
                client_type,
                // name: "ios",
                api_key: "AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc",
                context_client_name: "5",
                client: Client {
                    name: "IOS",
                    version: "19.09.3",
                    user_agent: Some("com.google.ios.youtube/19.09.3 (iPhone14,3; U; CPU iOS 15_6 like Mac OS X)"),
                    sdk: None,
                }
            },
        }
    }
}

