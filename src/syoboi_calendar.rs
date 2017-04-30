extern crate chrono;
extern crate hyper;
extern crate serde;
extern crate serde_xml_rs;

pub struct Client {
    client: hyper::client::Client,
}

pub struct CalChkRequest {
    pub days: Option<u32>,
}

impl Default for CalChkRequest {
    fn default() -> Self {
        Self { days: None }
    }
}

#[derive(Debug, Deserialize)]
pub struct CalChkResponse {
    #[serde(rename = "ProgItems")]
    pub prog_items: ProgItems,
}

#[derive(Debug, Deserialize)]
pub struct ProgItems {
    #[serde(rename = "ProgItem")]
    pub prog_items: Vec<ProgItem>,
}

#[derive(Debug, Deserialize)]
pub struct ProgItem {
    #[serde(rename = "TID")]
    pub tid: u32,
    #[serde(rename = "PID")]
    pub pid: u32,
    #[serde(rename = "StTime", deserialize_with="deserialize_time")]
    pub st_time: chrono::DateTime<chrono::Local>,
    #[serde(rename = "EdTime", deserialize_with="deserialize_time")]
    pub ed_time: chrono::DateTime<chrono::Local>,
    #[serde(rename = "ChName")]
    pub ch_name: String,
    #[serde(rename = "ChID")]
    pub ch_id: u32,
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

fn deserialize_time<D>(deserializer: D) -> Result<chrono::DateTime<chrono::Local>, D::Error>
    where D: serde::Deserializer
{
    use syoboi_calendar::serde::Deserialize;
    use syoboi_calendar::chrono::TimeZone;

    let s = String::deserialize(deserializer)?;
    // JST
    match chrono::FixedOffset::east(9 * 3600).datetime_from_str(&s, "%Y%m%d%H%M%S") {
        Ok(t) => Ok(t.with_timezone(&chrono::Local)),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}

impl ProgItem {
    pub fn start_time(&self) -> chrono::DateTime<chrono::Local> {
        self.st_time + chrono::Duration::seconds(self.st_offset)
    }

    pub fn end_time(&self) -> chrono::DateTime<chrono::Local> {
        self.ed_time + chrono::Duration::seconds(self.st_offset)
    }
}

#[derive(Debug)]
pub enum Error {
    Hyper(hyper::Error),
    Xml(serde_xml_rs::Error),
}
impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error::Hyper(e)
    }
}
impl From<serde_xml_rs::Error> for Error {
    fn from(e: serde_xml_rs::Error) -> Self {
        Error::Xml(e)
    }
}

const BASE_URL: &'static str = "http://cal.syoboi.jp";

impl Client {
    pub fn new(client: hyper::client::Client) -> Self {
        Self { client: client }
    }

    pub fn cal_chk(&self, params: &CalChkRequest) -> Result<CalChkResponse, Error> {
        let mut url = format!("{}/cal_chk.php", BASE_URL);
        if let Some(days) = params.days {
            url.push_str(&format!("?days={}", days));
        }
        let response = self.client
            .get("http://cal.syoboi.jp/cal_chk.php")
            .send()?;
        Ok(serde_xml_rs::deserialize(response)?)
    }
}
