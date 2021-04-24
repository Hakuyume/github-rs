use reqwest::{Response, StatusCode};
use serde::Deserialize;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("[{status:?}] {message:?} ({documentation_url})")]
    Api {
        status: StatusCode,
        message: String,
        documentation_url: Url,
    },
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Jsonwebtoken(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    ReqwestHeaderToStr(#[from] reqwest::header::ToStrError),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
}

impl Error {
    pub(crate) async fn check_status(response: Response) -> Result<Response, Self> {
        #[derive(Deserialize)]
        struct Payload {
            message: String,
            documentation_url: Url,
        }

        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else {
            let payload = response.json::<Payload>().await?;
            Err(Self::Api {
                status,
                message: payload.message,
                documentation_url: payload.documentation_url,
            })
        }
    }
}
