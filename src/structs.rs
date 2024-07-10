use std::cmp::Ordering::Equal;

use serde::{Deserialize, Serialize};

use crate::mime::Mime;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub playability_status: PlayabilityStatus,
    pub video_details: VideoDetails,
    pub streaming_data: StreamData,
}

impl Video {
    pub fn best_audio(&self) -> &VideoFormat {
        // prefer quality > acodec > br > ext
        self.all_formats().max_by(|a, b| {
            let a_quality = a.audio_quality;
            let b_quality = b.audio_quality;
            let a_acodec = a.mime_type.acodec();
            let b_acodec = b.mime_type.acodec();

            if a_quality.xor(b_quality).is_some() {
                return a_quality.cmp(&b_quality);
            } else if let (Some(aq), Some(bq)) = (a_quality, b_quality) {
                if aq != bq {
                    return aq.cmp(&bq);
                }
            } else if a_acodec.xor(b_acodec).is_some() {
                return a_quality.cmp(&b_quality);
            } else if let (Some(ac), Some(bc)) = (a_acodec, b_acodec) {
                if ac != bc {
                    return ac.cmp(&bc);
                }
            } else if a.bitrate != b.bitrate {
                return a.bitrate.cmp(&b.bitrate);
            } else if a.mime_type.format() != b.mime_type.format() {
                return a.mime_type.format().cmp(&b.mime_type.format());
            }
            Equal
        }).unwrap()
    }

    pub fn best_video(&self) -> &VideoFormat {
        // prefer quality > acodec > br > ext
        self.all_formats().max_by(|a, b| {
            let a_quality = a.audio_quality;
            let b_quality = b.audio_quality;
            let a_acodec = a.mime_type.acodec();
            let b_acodec = b.mime_type.acodec();

            if a_quality.xor(b_quality).is_some() {
                return a_quality.cmp(&b_quality);
            } else if let (Some(aq), Some(bq)) = (a_quality, b_quality) {
                if aq != bq {
                    return aq.cmp(&bq);
                }
            } else if a_acodec.xor(b_acodec).is_some() {
                return a_quality.cmp(&b_quality);
            } else if let (Some(ac), Some(bc)) = (a_acodec, b_acodec) {
                if ac != bc {
                    return ac.cmp(&bc);
                }
            } else if a.bitrate != b.bitrate {
                return a.bitrate.cmp(&b.bitrate);
            } else if a.mime_type.format() != b.mime_type.format() {
                return a.mime_type.format().cmp(&b.mime_type.format());
            }
            Equal
        }).unwrap()
    }

    pub fn all_formats(&self) -> impl Iterator<Item = &VideoFormat> {
        self.streaming_data.adaptive_formats
            .iter()
            .chain(self.streaming_data.formats.iter().flatten())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDetails {
    pub allow_ratings: bool,
    pub author: String,
    pub channel_id: String,
    pub is_crawlable: bool,
    pub is_live_content: bool,
    pub is_owner_viewing: bool,
    pub is_private: bool,
    pub is_unplugged_corpus: bool,
    pub keywords: Vec<String>,
    pub length_seconds: String,
    pub short_description: String,
    #[serde(rename = "thumbnail")]
    pub thumbnails: Thumbnails,
    pub title: String,
    pub video_id: String,
    pub view_count: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Thumbnails {
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayabilityStatus {
    status: String,
    playable_in_embed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamData {
    pub adaptive_formats: Vec<VideoFormat>,
    pub formats: Option<Vec<VideoFormat>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFormat {
    pub itag: u32,
    pub bitrate: u32,
    pub quality: Quality,
    pub mime_type: Mime,
    pub last_modified: String,
    pub signature_cipher: Option<String>,
    pub url: Option<String>,
    pub average_bitrate: Option<u32>,
    pub fps: Option<u32>,
    pub approx_duration_ms: Option<String>,
    pub content_length: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub index_range: Option<Range>,
    pub init_range: Option<Range>,
    pub projection_type: Option<String>,
    pub quality_label: Option<String>,

    // audio only
    pub audio_channels: Option<u8>,
    pub audio_quality: Option<AudioQuality>,
    pub audio_sample_rate: Option<String>,
    pub is_drc: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub end: String,
    pub start: String,
}

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudioQuality {
    #[default]
    #[serde(rename = "tiny")]
    Unknown,
    #[serde(rename = "AUDIO_QUALITY_ULTRALOW")]
    Ultralow,
    #[serde(rename = "AUDIO_QUALITY_LOW")]
    Low,
    #[serde(rename = "AUDIO_QUALITY_MEDIUM")]
    Medium,
    #[serde(rename = "AUDIO_QUALITY_HIGH")]
    High,
}

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    #[default]
    Tiny,
    Small,
    Medium,
    Large,
    HD720,
    HD1080,
    HD1440,
    HD2160,
    HD2880,
    HighRes,
}
