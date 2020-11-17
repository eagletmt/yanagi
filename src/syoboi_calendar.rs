#[derive(Debug, serde::Deserialize)]
struct Syobocal {
    #[serde(rename = "ProgItems")]
    prog_items: ProgItems,
}

#[derive(Debug, serde::Deserialize)]
struct ProgItems {
    #[serde(rename = "ProgItem")]
    prog_items: Vec<ProgItem>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ProgItem {
    #[serde(rename = "PID")]
    pub pid: i32,
    #[serde(rename = "TID")]
    pub tid: i32,
    #[serde(rename = "StTime", deserialize_with = "deserialize_time")]
    pub st_time: chrono::NaiveDateTime,
    #[serde(rename = "EdTime", deserialize_with = "deserialize_time")]
    pub ed_time: chrono::NaiveDateTime,
    #[serde(rename = "ChName")]
    pub ch_name: String,
    #[serde(rename = "ChID")]
    pub ch_id: i32,
    #[serde(rename = "Count")]
    pub count: String,
    #[serde(rename = "StOffset")]
    pub st_offset: i64,
    #[serde(rename = "SubTitle")]
    pub sub_title: String,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "ProgComment")]
    pub prog_comment: String,
}

fn deserialize_time<'de, D>(deserializer: D) -> Result<chrono::NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;
    let s = String::deserialize(deserializer)?;
    match chrono::NaiveDateTime::parse_from_str(&s, "%Y%m%d%H%M%S") {
        Ok(t) => Ok(t),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}

pub async fn cal_chk() -> Result<Vec<ProgItem>, anyhow::Error> {
    let body = reqwest::get("https://cal.syoboi.jp/cal_chk.php?days=7")
        .await?
        .error_for_status()?
        .text()
        .await?;
    let syobocal: Syobocal = serde_xml_rs::from_str(&body)?;
    Ok(syobocal.prog_items.prog_items)
}

#[derive(serde::Deserialize)]
struct TitleMedium {
    #[serde(rename = "Titles")]
    titles: std::collections::HashMap<u32, TitleMediumInfo>,
}
#[derive(serde::Deserialize)]
struct TitleMediumInfo {
    #[serde(rename = "Title")]
    title: String,
}

pub async fn title_medium(tid: u32) -> Result<Option<String>, anyhow::Error> {
    let mut resp: TitleMedium = reqwest::get(&format!(
        "https://cal.syoboi.jp/json.php?Req=TitleMedium&Tid={}",
        tid
    ))
    .await?
    .error_for_status()?
    .json()
    .await?;
    if let Some(info) = resp.titles.remove(&tid) {
        Ok(Some(info.title))
    } else {
        Ok(None)
    }
}
