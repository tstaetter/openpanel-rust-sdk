pub mod user;

use crate::{TrackerError, TrackerResult};
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::{Body, Response};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
enum TrackType {
    Decrement,
    Identify,
    Increment,
    #[default]
    Track,
}

impl Display for TrackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Tracker {
    api_url: String,
    client_id: String,
    client_secret: String,
    headers: HeaderMap,
    payload: Option<serde_json::Value>,
    disabled: bool,
}

impl Tracker {
    /// Create new tracker instance
    /// Load configuration from .env file
    pub fn try_new_from_env() -> TrackerResult<Self> {
        dotenvy::dotenv()?;

        let api_url = std::env::var("OPENPANEL_TRACK_URL")?;
        let client_id = std::env::var("OPENPANEL_CLIENT_ID")?;
        let client_secret = std::env::var("OPENPANEL_CLIENT_SECRET")?;

        Ok(Self {
            api_url,
            client_id,
            client_secret,
            headers: HeaderMap::new(),
            payload: None,
            disabled: false,
        })
    }

    /// Set default headers for tracker object
    pub fn with_default_headers(mut self) -> TrackerResult<Self> {
        self.headers.insert(
            HeaderName::from_str("Content-Type")?,
            "application/json".parse()?,
        );

        self.headers.insert(
            HeaderName::from_str("openpanel-client-id")?,
            self.client_id.parse()?,
        );

        self.headers.insert(
            HeaderName::from_str("openpanel-client-secret")?,
            self.client_secret.parse()?,
        );

        Ok(self)
    }

    /// Set custom header for tracker object
    pub fn with_header(mut self, key: String, value: String) -> TrackerResult<Self> {
        self.headers
            .insert(HeaderName::from_str(key.as_str())?, value.parse()?);

        Ok(self)
    }

    /// Set payload for tracker object
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);

        self
    }

    /// Disable sending events to OpenPanel
    pub fn disable(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub async fn track(
        &self,
        event: String,
        properties: HashMap<String, String>,
    ) -> TrackerResult<Response> {
        let payload = serde_json::json!({
          "type": TrackType::Track,
          "payload": {
            "name": event,
            "properties": properties
          }
        });

        self.send_request(payload).await
    }

    /// Identify user on OpenPanel
    pub async fn identify(&self, user: user::IdentifyUser) -> TrackerResult<Response> {
        let payload = serde_json::json!({
          "type": TrackType::Identify,
          "payload": user
        });

        self.send_request(payload).await
    }

    /// Decrement property value on OpenPanel
    pub async fn decrement(
        &self,
        profile_id: String,
        property: String,
        value: i64,
    ) -> TrackerResult<Response> {
        let payload = serde_json::json!({
          "type": TrackType::Decrement,
          "payload": {
            "profileId": profile_id,
            "property": property,
            "value": value
          }
        });

        self.send_request(payload).await
    }

    /// Decrement property value on OpenPanel
    pub async fn increment(
        &self,
        profile_id: String,
        property: String,
        value: i64,
    ) -> TrackerResult<Response> {
        let payload = serde_json::json!({
          "type": TrackType::Increment,
          "payload": {
            "profileId": profile_id,
            "property": property,
            "value": value
          }
        });

        self.send_request(payload).await
    }

    /// Actually send the request to the API
    async fn send_request(&self, payload: serde_json::Value) -> TrackerResult<Response> {
        if self.disabled {
            return Err(TrackerError::Disabled);
        }

        tracing::debug!("Sending request to {}", self.api_url);

        let client = reqwest::Client::new();
        let res = client
            .post(self.api_url.as_str())
            .body(Body::wrap(serde_json::to_string(&payload)?))
            .headers(self.headers.clone())
            .send()
            .await?;

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;
    use serde_json::json;

    #[test]
    fn can_set_default_headers() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;

        assert_eq!(
            tracker.headers.get("Content-Type").unwrap(),
            "application/json".parse::<HeaderValue>()?
        );
        assert_eq!(
            tracker.headers.get("openpanel-client-id").unwrap(),
            std::env::var("OPENPANEL_CLIENT_ID")
                .unwrap()
                .parse::<HeaderValue>()?
        );
        assert_eq!(
            tracker.headers.get("openpanel-client-secret").unwrap(),
            std::env::var("OPENPANEL_CLIENT_SECRET")
                .unwrap()
                .parse::<HeaderValue>()?
        );

        Ok(())
    }

    #[test]
    fn can_set_custom_header() -> anyhow::Result<()> {
        let tracker =
            Tracker::try_new_from_env()?.with_header("test".to_string(), "test".to_string())?;

        dbg!(&tracker.headers);

        assert_eq!(
            tracker.headers.get("test").unwrap(),
            "test".parse::<HeaderValue>()?
        );

        Ok(())
    }

    #[test]
    fn can_set_payload() -> anyhow::Result<()> {
        let payload = json!({
          "type": TrackType::Track,
          "payload": {
            "name": "test_event",
            "properties": {
              "name": "test"
            }
          }
        });
        let tracker = Tracker::try_new_from_env()?.with_payload(payload.clone());

        assert_eq!(tracker.payload, Some(payload));

        Ok(())
    }

    #[tokio::test]
    async fn can_send_request() -> anyhow::Result<()> {
        let payload = json!({
          "type": TrackType::Track,
          "payload": {
            "name": "test_event",
            "properties": {
              "name": "rust"
            }
          }
        });

        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let response = tracker.send_request(payload).await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn cannot_send_request_if_disabled() -> anyhow::Result<()> {
        let payload = json!({
          "type": TrackType::Track,
          "payload": {
            "name": "test_event",
            "properties": {
              "name": "rust"
            }
          }
        });

        let tracker = Tracker::try_new_from_env()?
            .with_default_headers()?
            .disable();
        let response = tracker.send_request(payload).await;

        assert!(response.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn can_track_event() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let mut properties = HashMap::new();

        properties.insert("name".to_string(), "rust".to_string());

        let response = tracker.track("test_event".to_string(), properties).await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn can_identify_user() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let mut properties = HashMap::new();

        properties.insert("name".to_string(), "test".to_string());

        let user = user::IdentifyUser {
            profile_id: "test_profile_id".to_string(),
            email: "".to_string(),
            first_name: "test".to_string(),
            last_name: "test".to_string(),
            properties,
        };

        let response = tracker.identify(user).await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn can_increment_property() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let response = tracker
            .increment(
                "test_profile_id".to_string(),
                "test_property".to_string(),
                1,
            )
            .await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn can_decrement_property() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let response = tracker
            .decrement(
                "test_profile_id".to_string(),
                "test_property".to_string(),
                1,
            )
            .await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }
}
