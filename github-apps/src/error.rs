use reqwest::Response;
use serde::Deserialize;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    RestApi(rest_api::Error),
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
    pub(crate) async fn check_rest_api_response(response: Response) -> Result<Response, Self> {
        #[derive(Deserialize)]
        struct Payload {
            #[serde(default)]
            message: Option<String>,
            #[serde(default)]
            errors: Option<Vec<rest_api::ErrorObject>>,
            #[serde(default)]
            documentation_url: Option<Url>,
        }

        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else {
            let text = response.text().await?;
            if let Ok(payload) = serde_json::from_str::<Payload>(&text) {
                Err(Self::RestApi(rest_api::Error {
                    status,
                    message: payload.message,
                    errors: payload.errors,
                    documentation_url: payload.documentation_url,
                }))
            } else {
                Err(Self::RestApi(rest_api::Error {
                    status,
                    message: Some(text),
                    errors: None,
                    documentation_url: None,
                }))
            }
        }
    }
}

// https://docs.github.com/en/rest/overview/resources-in-the-rest-api#client-errors
pub mod rest_api {
    use reqwest::StatusCode;
    use serde::Deserialize;
    use std::error;
    use std::fmt::{self, Display, Formatter};
    use url::Url;

    #[derive(Debug)]
    pub struct Error {
        pub status: StatusCode,
        pub message: Option<String>,
        pub errors: Option<Vec<ErrorObject>>,
        pub documentation_url: Option<Url>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ErrorObject {
        pub resource: String,
        pub field: String,
        pub code: ErrorCode,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ErrorCode {
        Missing,
        MissingField,
        Invalid,
        AlreadyExists,
        Unprocessable,
        Custom,
    }

    impl Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "[{}]", self.status)?;
            if let Some(message) = &self.message {
                write!(f, " \"{}\"", message)?;
            }
            if let Some(documentation_url) = &self.documentation_url {
                write!(f, " ({})", documentation_url)?;
            }
            Ok(())
        }
    }

    impl error::Error for Error {}
}
