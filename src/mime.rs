use std::{fmt, str::FromStr};

use serde::de::{self, Deserialize, Deserializer, Visitor};

use crate::errors::Error;
use crate::utils::between;

/// Mime enum type for stream data. Streams can come in three types:
/// audio-only, video-only, video with audio.
/// For each type, codecs are given for both audio and video if they exist, along with the
/// format/container of the stream.
///
/// The codecs and format implement ord so you can compare them to see which one is better.
/// For [`Format`], it is best to keep it to audio-audio or video-video comparisons and not
/// audio-video, which gets tricky, depending if you prefer audio or video formats more.
#[derive(Debug, serde::Serialize)]
pub enum Mime {
    Audio(Format, Acodec),
    Video(Format, Vcodec, Option<Acodec>),
}

impl Mime {
    /// Gets the audio codec of the mime
    #[must_use]
    pub fn acodec(&self) -> Option<Acodec> {
        match self {
            Mime::Audio(_, acodec) => Some(*acodec),
            Mime::Video(_, _, acodec) => *acodec,
        }
    }

    /// Gets the video codec of the mime
    #[must_use]
    pub fn vcodec(&self) -> Option<Vcodec> {
        match self {
            Mime::Audio(_, _) => None,
            Mime::Video(_, vcodec, _) => Some(*vcodec),
        }
    }

    /// Gets the format of the mime
    #[must_use]
    pub fn format(&self) -> Format {
        match self {
            Mime::Audio(format, _) | Mime::Video(format, _, _) => *format,
        }
    }
}

impl FromStr for Mime {
    type Err = Error;

    fn from_str(input: &str) -> Result<Mime, Self::Err> {
        let input = input.trim();

        let format = between(input, "/", ";").parse::<Format>()?;
        let codecs = between(input, "\"", "\"");
        let split = input
            .find('/')
            .ok_or(Error::MimeParse("/", String::new()))?;

        match &input[..split] {
            "audio" => Ok(Mime::Audio(format, codecs.parse::<Acodec>()?)),
            "video" => {
                let acodec_start = codecs.find(',');
                if let Some(pos) = acodec_start {
                    let vcodec = codecs[..pos].parse::<Vcodec>()?;
                    let acodec = codecs[(pos + 1)..].trim().parse::<Acodec>()?;
                    Ok(Mime::Video(format, vcodec, Some(acodec)))
                } else {
                    Ok(Mime::Video(format, codecs.parse::<Vcodec>()?, None))
                }
            }
            _ => Err(Error::MimeParse("valid mime string", input[..split].to_owned())),
        }
    }
}

struct MimeVisitor;

impl<'de> Visitor<'de> for MimeVisitor {
    type Value = Mime;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a valid audio or video mimetype string")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Mime::from_str(s).map_err(|e| de::Error::custom(e))
    }
}

impl<'de> Deserialize<'de> for Mime {
    fn deserialize<D>(deserializer: D) -> Result<Mime, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(MimeVisitor)
    }
}

/// Format/container of a mime
#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Format {
    Webm,
    MP4,
}

impl FromStr for Format {
    type Err = Error;

    fn from_str(input: &str) -> Result<Format, Error> {
        if input.starts_with("mp4") {
            Ok(Format::MP4)
        } else if input.starts_with("webm") {
            Ok(Format::Webm)
        } else {
            Err(Error::MimeParse("format", input.to_owned()))
        }
    }
}

/// Video codec of a mime
#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Vcodec {
    AVC,
    AV1,
    // prefer VP9 for now over AV1 due to lack of support for AV1
    VP9,
}

impl FromStr for Vcodec {
    type Err = Error;

    fn from_str(input: &str) -> Result<Vcodec, Error> {
        if input.starts_with("av01") {
            Ok(Vcodec::AV1)
        } else if input.starts_with("vp9") {
            Ok(Vcodec::VP9)
        } else if input.starts_with("avc1") {
            Ok(Vcodec::AVC)
        } else {
            Err(Error::MimeParse("video codec", input.to_owned()))
        }
    }
}

/// Audio codec of a mime
#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Acodec {
    MP4A,
    AAC,
    Vorbis,
    Opus,
    // FLAC,
}

impl FromStr for Acodec {
    type Err = Error;

    fn from_str(input: &str) -> Result<Acodec, Error> {
        if input.starts_with("opus") {
            Ok(Acodec::Opus)
        } else if input.starts_with("vorbis") {
            Ok(Acodec::Vorbis)
        } else if input.starts_with("mp4a.40.2") {
            // special case of mp4a
            Ok(Acodec::AAC)
        } else if input.starts_with("mp4a") {
            Ok(Acodec::MP4A)
        } else {
            Err(Error::MimeParse("audio codec", input.to_owned()))
        }
    }
}
