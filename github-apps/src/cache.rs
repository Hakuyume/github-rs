use crate::{AccessToken, App, Error, Installation};
use chrono::{Duration, Utc};
use reqwest::Client;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Cache {
    inner: Arc<Inner>,
}

struct Inner {
    app: App,
    cache: Mutex<HashMap<u64, AccessToken>>,
}

impl Cache {
    pub fn new(app: App) -> Self {
        Self {
            inner: Arc::new(Inner {
                app,
                cache: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn app(&self) -> &App {
        &self.inner.app
    }

    pub async fn access_token(
        &self,
        client: &Client,
        installation: &Installation,
    ) -> Result<AccessToken, Error> {
        let mut cache = self.inner.cache.lock().await;
        let access_token = match cache.entry(installation.id) {
            Entry::Occupied(entry) => {
                let access_token = entry.into_mut();
                if access_token.expires_at < Utc::now() + Duration::seconds(60) {
                    *access_token = self.inner.app.access_token(client, installation).await?;
                }
                access_token
            }
            Entry::Vacant(entry) => {
                entry.insert(self.inner.app.access_token(client, installation).await?)
            }
        };
        Ok(access_token.clone())
    }
}
