use serde::Deserialize;

// Generated using https://transform.tools/json-to-rust-serde
// Not public facing but are used instead of serde_json::Value
// for performance and static checking.

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearch {
    contents: Contents,
}

impl WebSearch {
    pub fn queries(&self) -> Vec<String> {
        self.contents
            .two_column_search_results_renderer
            .primary_contents
            .section_list_renderer
            .contents
            .iter()
            .find(|x| x.item_section_renderer.is_some())
            .unwrap()
            .item_section_renderer
            .as_ref()
            .unwrap()
            .contents
            .iter()
            .filter_map(|x| x.video_renderer.as_ref().map(|x| x.video_id.to_string()))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contents {
    pub two_column_search_results_renderer: TwoColumnSearchResultsRenderer,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TwoColumnSearchResultsRenderer {
    pub primary_contents: PrimaryContents,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrimaryContents {
    pub section_list_renderer: SectionListRenderer,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SectionListRenderer {
    pub contents: Vec<Content>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Content {
    pub item_section_renderer: Option<ItemSectionRenderer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemSectionRenderer {
    pub contents: Vec<Content2>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Content2 {
    pub video_renderer: Option<VideoRenderer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoRenderer {
    pub video_id: String,
}
