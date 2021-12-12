use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct Funscript {
    #[serde(rename = "version")]
    pub version: String,

    #[serde(rename = "inverted")]
    pub inverted: bool,

    #[serde(rename = "range")]
    pub range: i64,

    #[serde(rename = "actions")]
    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Action {
    #[serde(rename = "pos")]
    pub pos: u8,

    #[serde(rename = "at", deserialize_with = "duration_from_millis")]
    pub at: Duration,
}

fn duration_from_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    u64::deserialize(deserializer).map(|millis| Duration::from_millis(millis))
}
