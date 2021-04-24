mod cache;
mod error;

pub use cache::Cache;
use chrono::{DateTime, Duration, Utc};
pub use error::Error;
use hyper::header::{Link, RelationType};
pub use jsonwebtoken::EncodingKey;
use jsonwebtoken::{Algorithm, Header};
use reqwest::header::{ACCEPT, LINK};
pub use reqwest::Client;
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
pub use url::Url;

pub struct App {
    endpoint: Url,
    id: u64,
    private_key: EncodingKey,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Installation {
    pub id: u64,
    pub access_tokens_url: Url,
    pub repositories_url: Url,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

impl App {
    pub fn new(endpoint: Url, id: u64, private_key: EncodingKey) -> Self {
        Self {
            endpoint,
            id,
            private_key,
        }
    }

    // https://docs.github.com/en/developers/apps/authenticating-with-github-apps#authenticating-as-a-github-app
    fn jwt(&self) -> Result<String, Error> {
        #[derive(Serialize)]
        struct Payload {
            iat: i64,
            exp: i64,
            iss: u64,
        }

        let iat = Utc::now();
        Ok(jsonwebtoken::encode(
            &Header::new(Algorithm::RS256),
            &Payload {
                iat: (iat - Duration::seconds(60)).timestamp(),
                exp: (iat + Duration::seconds(10 * 60)).timestamp(),
                iss: self.id,
            },
            &self.private_key,
        )?)
    }

    // https://docs.github.com/en/rest/reference/apps#list-installations-for-the-authenticated-app
    pub async fn installations(&self, client: &Client) -> Result<Vec<Installation>, Error> {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .unwrap()
            .push("app")
            .push("installations");
        pagination(client, url, |builder| {
            Ok(builder
                .header(ACCEPT, "application/vnd.github.v3+json")
                .bearer_auth(self.jwt()?))
        })
        .await
    }

    // https://docs.github.com/en/rest/reference/apps#create-an-installation-access-token-for-an-app
    pub async fn access_token(
        &self,
        client: &Client,
        installation: &Installation,
    ) -> Result<AccessToken, Error> {
        Ok(Error::check_status(
            client
                .post(installation.access_tokens_url.clone())
                .header(ACCEPT, "application/vnd.github.v3+json")
                .bearer_auth(self.jwt()?)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    // https://docs.github.com/en/rest/reference/apps#get-a-repository-installation-for-the-authenticated-app
    pub async fn repository_installation(
        &self,
        client: &Client,
        owner: &str,
        repo: &str,
    ) -> Result<Installation, Error> {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .unwrap()
            .push("repos")
            .push(owner)
            .push(repo)
            .push("installation");
        Ok(Error::check_status(
            client
                .get(url)
                .header(ACCEPT, "application/vnd.github.v3+json")
                .bearer_auth(self.jwt()?)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }
}

async fn pagination<F, T>(client: &Client, url: Url, mut f: F) -> Result<Vec<T>, Error>
where
    F: FnMut(RequestBuilder) -> Result<RequestBuilder, Error>,
    T: DeserializeOwned,
{
    let mut items = Vec::new();
    let mut url = Some(url);
    while let Some(u) = url.take() {
        let response = Error::check_status(f(client.get(u))?.send().await?).await?;
        if let Some(link) = response.headers().get(LINK) {
            url = link
                .to_str()?
                .parse::<Link>()?
                .values()
                .iter()
                .find_map(|link_value| {
                    link_value.rel().and_then(|rel| {
                        rel.contains(&RelationType::Next)
                            .then(|| Url::parse(link_value.link()))
                    })
                })
                .transpose()?;
        }
        items.append(&mut response.json().await?);
    }
    Ok(items)
}
