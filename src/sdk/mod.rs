//! OpenPanel SDK for tracking events
//!
//! # Example
//!
//! ```rust
//! use openpanel_sdk::sdk::Tracker;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
//!     let mut properties = HashMap::new();
//!
//!     properties.insert("name".to_string(), "rust".to_string());
//!
//!     tracker.track("test".to_string(), Some(properties), None).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! or apply filter
//!
//! ```rust
//! use openpanel_sdk::sdk::Tracker;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let filter = |properties: HashMap<String, String>| properties.contains_key("not-existing");
//!     let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
//!     let mut properties = HashMap::new();
//!
//!     properties.insert("name".to_string(), "rust".to_string());
//!
//!     // will return error because properties doesn't contain key "not-existing"
//!     let result = tracker.track("test".to_string(), Some(properties), Some(&filter)).await;
//!
//!     assert!(result.is_err());
//!
//!     Ok(())
//! }
//! ```
pub mod user;

use crate::{TrackerError, TrackerResult};
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::{Body, Response};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

/// Type of event to track
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
enum TrackType {
    /// Decrement property value on OpenPanel
    Decrement,
    /// Identify property value on OpenPanel
    Identify,
    /// Increment property value on OpenPanel
    Increment,
    /// Track event on OpenPanel
    #[default]
    Track,
}

impl Display for TrackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// OpenPanel SDK for tracking events
#[derive(Debug)]
pub struct Tracker {
    api_url: String,
    client_id: String,
    client_secret: String,
    headers: HeaderMap,
    global_props: HashMap<String, String>,
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
            global_props: HashMap::new(),
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

    /// Set a custom header for a tracker object.
    /// Use this to set custom headers used for e.g. geo location
    pub fn with_header(mut self, key: String, value: String) -> TrackerResult<Self> {
        self.headers
            .insert(HeaderName::from_str(key.as_str())?, value.parse()?);

        Ok(self)
    }

    /// Set global properties for tracker object. Global properties are added to every
    /// `track` and `identify` event sent.
    pub fn with_global_properties(mut self, properties: HashMap<String, String>) -> Self {
        self.global_props = properties;

        self
    }

    /// Disable sending events to OpenPanel
    pub fn disable(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Track event on OpenPanel
    ///
    /// # Parameters:
    /// - event [String]: The event name
    /// - properties [Option<HashMap<String, String>>]: Additional properties to send with the event
    /// - filter [Option<&dyn Fn(HashMap<String, String>) -> bool>]: If provided, the filter fn will
    ///     be applied onto the payload. If the result is true, the event won't be sent
    pub async fn track(
        &self,
        event: String,
        properties: Option<HashMap<String, String>>,
        filter: Option<&dyn Fn(HashMap<String, String>) -> bool>,
    ) -> TrackerResult<Response> {
        if let Some(filter) = filter {
            if filter(self.create_properties_with_globals(properties.clone())) {
                return Err(TrackerError::Filtered);
            }
        }

        let properties = self.create_properties_with_globals(properties);
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
    pub async fn identify(&self, mut user: user::IdentifyUser) -> TrackerResult<Response> {
        user.properties = self.create_properties_with_globals(Some(user.properties));

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

    pub async fn revenue(
        &self,
        amount: i64,
        properties: Option<HashMap<String, String>>,
    ) -> TrackerResult<Response> {
        let local_props = HashMap::from([("amount".to_string(), amount.to_string())]);
        let mut properties = self.create_properties_with_globals(properties.clone());

        properties.extend(local_props);

        let payload = serde_json::json!({
          "type": TrackType::Track,
          "payload": {
            "name": "revenue",
            "amount": amount,
            "properties": properties
          }
        });

        self.send_request(payload).await
    }

    pub async fn fetch_device_id(&self) -> TrackerResult<String> {
        if self.disabled {
            return Err(TrackerError::Disabled);
        }

        let url = format!("{}/device-id", self.api_url);
        tracing::debug!("Sending request to {}", url);

        let client = reqwest::Client::new();
        let res = client
            .get(url.as_str())
            .headers(self.headers.clone())
            .send()
            .await?;
        let body = res.text().await?;
        let json = serde_json::from_str::<HashMap<String, String>>(&body)?;
        let id = if !json.contains_key("deviceId") {
            return Ok("".to_string());
        } else {
            json.get("deviceId").unwrap().to_string()
        };

        Ok(id)
    }

    /// Extend given properties with global properties
    fn create_properties_with_globals(
        &self,
        properties: Option<HashMap<String, String>>,
    ) -> HashMap<String, String> {
        if let Some(mut properties) = properties {
            properties.extend(self.global_props.clone());
            properties
        } else {
            self.global_props.clone()
        }
    }

    /// Actually send the request to the API
    async fn send_request(&self, payload: serde_json::Value) -> TrackerResult<Response> {
        if self.disabled {
            return Err(TrackerError::Disabled);
        }

        tracing::debug!("Sending request to {}", self.api_url);
        tracing::debug!(
            "Sending payload {:?}",
            serde_json::to_string_pretty(&payload)?
        );

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

        assert_eq!(
            tracker.headers.get("test").unwrap(),
            "test".parse::<HeaderValue>()?
        );

        Ok(())
    }

    #[test]
    fn can_create_properties_with_globals() -> anyhow::Result<()> {
        let properties = HashMap::from([("test".to_string(), "test".to_string())]);
        let tracker = Tracker::try_new_from_env()?.with_global_properties(properties.clone());
        let properties_with_globals =
            tracker.create_properties_with_globals(Some(properties.clone()));

        assert_eq!(tracker.global_props, properties_with_globals);

        Ok(())
    }

    #[test]
    fn can_set_global_properties() -> anyhow::Result<()> {
        let properties = HashMap::from([("test".to_string(), "test".to_string())]);
        let tracker = Tracker::try_new_from_env()?.with_global_properties(properties.clone());

        assert_eq!(tracker.global_props, properties);

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

        let response = tracker
            .track("test_event".to_string(), Some(properties), None)
            .await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn can_filter_track_event() -> anyhow::Result<()> {
        let filter = |properties: HashMap<String, String>| properties.contains_key("name");
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let mut properties = HashMap::new();

        properties.insert("name".to_string(), "rust".to_string());

        let response = tracker
            .track("test_event".to_string(), Some(properties), Some(&filter))
            .await;

        assert!(response.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn can_identify_user() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let mut properties = HashMap::new();

        properties.insert("name".to_string(), "rust".to_string());

        let user = user::IdentifyUser {
            profile_id: "test_profile_id".to_string(),
            email: "rust@test.com".to_string(),
            first_name: "Rust".to_string(),
            last_name: "Rust".to_string(),
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

    #[tokio::test]
    async fn can_track_revenue() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
        let response = tracker.revenue(100, None).await?;

        assert_eq!(response.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn can_fetch_device_id() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?
            .with_default_headers()?
            .with_header("user-agent".to_string(), "some".to_string())?;
        let id = tracker.fetch_device_id().await?;

        assert!(!id.is_empty());

        Ok(())
    }
}
