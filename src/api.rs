use serde::Deserialize;

#[derive(Clone)]
pub struct HNApi {
    client: reqwest::Client,
    base: reqwest::Url,
}
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct HNItem {
    pub id: u64,
    deleted: Option<bool>,
    r#type: Option<String>,
    by: Option<String>,
    time: Option<u64>,
    text: Option<String>,
    dead: Option<String>,
    parent: Option<u64>,
    kids: Option<Vec<u64>>,
    url: Option<String>,
    score: Option<u64>,
    title: Option<String>,
    descendants: Option<u64>,
}
impl HNItem {
    fn url(&self) -> String {
        format!("https://news.ycombinator.com/item?id={}", self.id)
    }
    pub fn format_story(&self) -> anyhow::Result<String> {
        anyhow::ensure!(self.r#type.as_deref() == Some("story"), "Not a story");
        Ok(format!(
            "{} {} ({} votes, {} comments)",
            self.title.as_deref().unwrap_or_default(),
            self.url(),
            self.score.unwrap_or_default(),
            self.descendants.unwrap_or_default()
        ))
    }
    pub fn selected(&self) -> bool {
        self.descendants.unwrap_or_default() > 200 || self.score.unwrap_or_default() > 200
    }
}
impl HNApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base: reqwest::Url::parse("https://hacker-news.firebaseio.com/v0/").unwrap(),
        }
    }
    pub async fn top(&self) -> anyhow::Result<Vec<u64>> {
        Ok(self
            .client
            .get(self.base.join("topstories.json")?)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<u64>>()
            .await?)
    }
    pub async fn item(&self, id: u64) -> anyhow::Result<HNItem> {
        let item: HNItem = self
            .client
            .get(self.base.join("item/")?.join(&format!("{}.json", id))?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(item)
    }
}
