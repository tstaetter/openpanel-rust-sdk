mod sdk;

pub type TrackerResult<T> = Result<T, TrackerError>;

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("Error loading env variables: {0:?}")]
    EnvVar(#[from] dotenvy::Error),
    #[error("Error loading env variables: {0:?}")]
    Env(#[from] std::env::VarError),
    #[error("Not Authorized")]
    NotAuthorized,
    #[error("Too many requests")]
    TooManyRequests,
    #[error("Internal error")]
    Internal,
    #[error("Request error: {0:?}")]
    Request(#[from] reqwest::Error),
    #[error("Error serializing payload: {0:?}")]
    Serializing(#[from] serde_json::Error),
    #[error("Invalid header name: {0:?}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),
    #[error("Invalid header value: {0:?}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),
}
