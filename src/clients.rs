use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use serde_json::{json, Map};

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/74.0.3729.157 Safari/537.36";

#[derive(Debug, PartialEq)]
pub enum ClientType {
    Web,
    WebEmbedded,
    WebCreator,
    Android,
    AndroidEmbedded,
    AndroidCreator,
    Ios,
    IosEmbedded,
    IosCreator,
}

#[derive(Debug, Serialize)]
struct Client {
    #[serde(rename(serialize = "clientName"))]
    name: &'static str,
    #[serde(rename(serialize = "clientVersion"))]
    version: &'static str,

    #[serde(rename(serialize = "androidSdkVersion"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    sdk: Option<u8>,

    #[serde(skip_serializing)]
    user_agent: Option<&'static str>,
}

#[derive(Debug)]
pub struct ClientConfig {
    client_type: ClientType,
    api_key: &'static str,
    context_client_name: &'static str,
    client: Client,
}

impl ClientConfig {
    pub fn context_json(&self) -> serde_json::Value {
        let mut client = serde_json::to_value(&self.client).unwrap();
        let map = client.as_object_mut().unwrap();
        map.insert("hl".to_owned(), "en".to_owned().into());
        if self.is_base() {
            map.insert("clientScreen".to_owned(), "EMBED".into());
        }

        if self.is_base() || self.is_embed() {
            return json!({
                "client": client,
                "thirdParty": {
                    "embedUrl": "https://www.youtube.com/",
                },
            });
        }

        json!({"client": client})
    }

    pub fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert(
            "x-youtube-client-name",
            HeaderValue::from_static(self.context_client_name),
        );
        headers.insert(
            "x-youtube-client-version",
            HeaderValue::from_static(self.client.version),
        );
        headers.insert(
            "user-agent",
            HeaderValue::from_static(self.client.user_agent.unwrap_or(DEFAULT_USER_AGENT)),
        );
        let origin = format!(r"https:\\{}", self.hostname());
        headers.insert("origin", HeaderValue::from_str(&origin).unwrap());

        headers
    }

    pub fn new(client_type: ClientType) -> Self {
        match client_type {
            ClientType::Web => ClientConfig {
                client_type,
                api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
                context_client_name: "1",
                client: Client {
                    name: "WEB",
                    version: "2.20220801.00.00",
                    user_agent: None,
                    sdk: None,
                }
            },
            ClientType::WebEmbedded => ClientConfig {
                client_type,
                api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
                context_client_name: "56",
                client: Client {
                    name: "WEB_EMBEDDED_PLAYER",
                    version: "1.20220731.00.00",
                    user_agent: None,
                    sdk: None,
                }
            },
            ClientType::WebCreator => ClientConfig {
                client_type,
                api_key: "AIzaSyBUPetSUmoZL-OhlxA7wSac5XinrygCqMo",
                context_client_name: "62",
                client: Client {
                    name: "WEB_CREATOR",
                    version: "1.20220726.00.00",
                    user_agent: None,
                    sdk: None,
                }
            },
            ClientType::Android => ClientConfig {
                client_type,
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
                api_key: "AIzaSyCjc_pVEDi4qsv5MtC2dMXzpIaDoRFLsxw",
                context_client_name: "55",
                client: Client {
                    name: "ANDROID_EMBEDDED_PLAYER",
                    version: "19.09.37",
                    sdk: Some(30),
                    user_agent: Some("com.google.android.youtube/19.09.37 (Linux; U; Android 11) gzip"),
                }
            },
            ClientType::AndroidCreator => ClientConfig {
                client_type,
                api_key: "AIzaSyD_qjV8zaaUMehtLkrKFgVeSX_Iqbtyws8",
                context_client_name: "14",
                client: Client {
                    name: "ANDROID_CREATOR",
                    version: "22.30.100",
                    sdk: Some(30),
                    user_agent: Some("com.google.android.apps.youtube.creator/22.30.100 (Linux; U; Android 11) gzip"),
                }
            },
            ClientType::Ios => ClientConfig {
                client_type,
                api_key: "AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc",
                context_client_name: "5",
                client: Client {
                    name: "IOS",
                    version: "19.09.3",
                    user_agent: Some("com.google.ios.youtube/19.09.3 (iPhone14,3; U; CPU iOS 15_6 like Mac OS X)"),
                    sdk: None,
                }
            },
            ClientType::IosEmbedded => ClientConfig {
                client_type,
                api_key: "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8",
                context_client_name: "26",
                client: Client {
                    name: "IOS_MESSAGES_EXTENSION",
                    version: "19.09.3",
                    user_agent: Some("com.google.ios.youtube/19.09.3 (iPhone14,3; U; CPU iOS 15_6 like Mac OS X)"),
                    sdk: None,
                }
            },
            ClientType::IosCreator => ClientConfig {
                client_type,
                api_key: "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8",
                context_client_name: "15",
                client: Client {
                    name: "IOS_CREATOR",
                    version: "22.33.101",
                    user_agent: Some("com.google.ios.ytcreator/22.33.101 (iPhone14,3; U; CPU iOS 15_6 like Mac OS X)"),
                    sdk: None,
                }
            },
        }
    }
}
