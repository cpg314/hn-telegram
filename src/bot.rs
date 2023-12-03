use std::collections::HashSet;
use std::path::PathBuf;

use futures::stream::{self, StreamExt, TryStreamExt};
use log::*;
use teloxide::requests::Requester;

use super::{HNApi, HNItem};

pub struct HNBot {
    hn: HNApi,
    telegram: teloxide::Bot,
    marked: HashSet<u64>,
    recipient: teloxide::types::Recipient,
}
impl HNBot {
    pub fn new(telegram: teloxide::Bot, chat_id: i64) -> Self {
        let marked = Self::cache_path()
            .and_then(|path| {
                path.exists().then(|| {
                    std::fs::read_to_string(path)
                        .map_err(anyhow::Error::new)
                        .and_then(|s| {
                            serde_json::from_str::<HashSet<u64>>(&s).map_err(anyhow::Error::new)
                        })
                        .map_err(|e| {
                            warn!("Failed to deserialize from cache: {}", e);
                            e
                        })
                        .ok()
                })
            })
            .flatten()
            .unwrap_or_default();
        info!("{} marked stories loaded", marked.len());
        Self {
            hn: HNApi::new(),
            telegram,
            marked,
            recipient: teloxide::types::Recipient::Id(teloxide::types::ChatId(chat_id)),
        }
    }
    fn save(&self) -> anyhow::Result<()> {
        Ok(std::fs::write(
            Self::cache_path().ok_or_else(|| anyhow::anyhow!("Cache path not found"))?,
            serde_json::to_string(&self.marked)?,
        )?)
    }
    fn cache_path() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join("hn-telegram.json"))
    }
    pub async fn refresh_and_send(&mut self) -> anyhow::Result<()> {
        let items = self.refresh().await?;
        if !items.is_empty() {
            info!("Sending notifications for {} new items", items.len());
        }
        for item in items {
            self.telegram
                .send_message(self.recipient.clone(), item.format_story()?)
                .await?;
        }
        self.save()?;
        Ok(())
    }
    async fn refresh(&mut self) -> anyhow::Result<Vec<HNItem>> {
        info!("Refreshing HN stories");
        let mut top = self.hn.top().await?;
        top.truncate(10);
        let top: HashSet<u64> = top.into_iter().collect();
        info!("Got {} top stories", top.len());
        let top: HashSet<u64> = top.difference(&self.marked).copied().collect();
        info!(
            "{} of these top stories not yet marked, retrieving...",
            top.len()
        );
        let mut items: Vec<HNItem> = stream::iter(top)
            .map(|id| {
                let hn = self.hn.clone();
                async move { hn.item(id).await }
            })
            .buffer_unordered(8)
            .try_collect()
            .await?;
        items.retain(|i| i.selected());
        info!("{} item(s) newly selected", items.len());
        debug!("{:?}", items);
        self.marked.extend(items.iter().map(|x| x.id));

        Ok(items)
    }
}
