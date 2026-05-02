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
//!     tracker.track("test".to_string(), None, Some(properties), None, None).await?;
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
//!     let filter = |properties: HashMap<String, String>| properties.contains_key("name");
//!     let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
//!     let mut properties = HashMap::new();
//!
//!     properties.insert("name".to_string(), "rust".to_string());
//!
//!     // will return error because properties contain key "name"
//!     let result = tracker.track("test".to_string(), None, Some(properties), None, Some(&filter)).await;
//!
//!     assert!(result.is_err());
//!
//!     Ok(())
//! }
//! ```
pub mod group;
pub mod user;

use crate::{TrackerError, TrackerResult};
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::{Body, Response};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::RwLock;

/// Type of event to track
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
enum TrackType {
    /// Decrement property value on OpenPanel
    Decrement,
    /// Create or update a group entity on OpenPanel
    Group,
    /// Identify property value on OpenPanel
    Identify,
    /// Increment property value on OpenPanel
    Increment,
    /// Link a profile to one or more groups on OpenPanel
    AssignGroup,
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
    groups: RwLock<Vec<String>>,
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
            groups: RwLock::new(Vec::new()),
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
    /// - profile_id [Option<String>]: Optional profile ID
    /// - properties [Option<HashMap<String, String>>]: Additional properties to send with the event
    /// - groups [Option<Vec<String>>]: Per-event group IDs to attach (merged with persistent groups)
    /// - filter [Option<&dyn Fn(HashMap<String, String>) -> bool>]: If provided, the filter fn will
    ///   be applied onto the payload. If the result is true, the event won't be sent
    pub async fn track(
        &self,
        event: String,
        profile_id: Option<String>,
        properties: Option<HashMap<String, String>>,
        groups: Option<Vec<String>>,
        filter: Option<&dyn Fn(HashMap<String, String>) -> bool>,
    ) -> TrackerResult<Response> {
        if let Some(filter) = filter {
            if filter(self.create_properties_with_globals(properties.clone())) {
                return Err(TrackerError::Filtered);
            }
        }

        let properties = self.create_properties_with_globals(properties);

        // Merge persistent groups with per-event groups
        let merged_groups = {
            let persistent = self.groups.read().unwrap();
            match groups {
                Some(ref event_groups) => {
                    let mut merged = persistent.clone();
                    for g in event_groups {
                        if !merged.contains(g) {
                            merged.push(g.clone());
                        }
                    }
                    if merged.is_empty() {
                        None
                    } else {
                        Some(merged)
                    }
                }
                None => {
                    if persistent.is_empty() {
                        None
                    } else {
                        Some(persistent.clone())
                    }
                }
            }
        };

        let mut payload = serde_json::json!({
            "type": TrackType::Track,
            "payload": {
                "profileId": profile_id,
                "name": event,
                "properties": properties
            }
        });

        if let Some(ref g) = merged_groups {
            payload["payload"]["groups"] = serde_json::json!(g);
        }

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

    /// Create or update a group entity on OpenPanel.
    ///
    /// Use this to sync dynamic group metadata (plan, seats, MRR, etc.) from
    /// your own data. Call it on login or when group properties change — not on
    /// every request.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openpanel_sdk::sdk::{group::Group, Tracker};
    /// use std::collections::HashMap;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    ///     let group = Group {
    ///         id: "org_acme".to_string(),
    ///         group_type: "company".to_string(),
    ///         name: "Acme Inc".to_string(),
    ///         properties: HashMap::from([("plan".to_string(), "enterprise".to_string())]),
    ///     };
    ///     tracker.upsert_group(group).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn upsert_group(&self, group: group::Group) -> TrackerResult<Response> {
        let payload = serde_json::json!({
            "type": TrackType::Group,
            "payload": group
        });

        self.send_request(payload).await
    }

    /// Link the current profile to a single group ID.
    ///
    /// Sends an `assign_group` event to OpenPanel and persists the group ID on
    /// the SDK instance so that all subsequent `track()` calls automatically
    /// include it.
    ///
    /// The group does not need to exist yet — events will be tagged with the
    /// group ID and the group can be created later via `upsert_group` or in the
    /// dashboard.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openpanel_sdk::sdk::Tracker;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    ///     tracker.set_group("user_123".to_string(), "org_acme".to_string()).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_group(&self, profile_id: String, group_id: String) -> TrackerResult<Response> {
        // Persist the group ID on the tracker instance
        {
            let mut groups = self.groups.write().unwrap();
            if !groups.contains(&group_id) {
                groups.push(group_id.clone());
            }
        }

        // Send the assign_group event
        let payload = serde_json::json!({
            "type": TrackType::AssignGroup,
            "payload": {
                "profileId": profile_id,
                "groups": [group_id]
            }
        });

        self.send_request(payload).await
    }

    /// Link the current profile to multiple group IDs at once.
    ///
    /// Sends a single `assign_group` event containing all provided group IDs
    /// and persists them on the SDK instance so that all subsequent `track()`
    /// calls automatically include them.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openpanel_sdk::sdk::Tracker;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    ///     tracker
    ///         .set_groups(
    ///             "user_123".to_string(),
    ///             vec!["org_acme".to_string(), "team_engineering".to_string()],
    ///         )
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_groups(
        &self,
        profile_id: String,
        group_ids: Vec<String>,
    ) -> TrackerResult<Response> {
        // Persist the group IDs on the tracker instance
        {
            let mut groups = self.groups.write().unwrap();
            for gid in &group_ids {
                if !groups.contains(gid) {
                    groups.push(gid.clone());
                }
            }
        }

        // Send the assign_group event
        let payload = serde_json::json!({
            "type": TrackType::AssignGroup,
            "payload": {
                "profileId": profile_id,
                "groups": group_ids
            }
        });

        self.send_request(payload).await
    }

    /// Clear all persisted state on the tracker, including groups.
    ///
    /// Always call this on logout to prevent a new user from inheriting the
    /// previous user's groups.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openpanel_sdk::sdk::Tracker;
    ///
    /// fn handle_logout(tracker: &Tracker) {
    ///     tracker.clear();
    /// }
    /// ```
    pub fn clear(&self) {
        let mut groups = self.groups.write().unwrap();
        groups.clear();
    }

    pub async fn revenue(
        &self,
        profile_id: Option<String>,
        amount: i64,
        properties: Option<HashMap<String, String>>,
    ) -> TrackerResult<Response> {
        let local_props = HashMap::from([("__revenue".to_string(), amount.to_string())]);
        let mut properties = self.create_properties_with_globals(properties.clone());

        properties.extend(local_props);

        self.track(
            "revenue".to_string(),
            profile_id,
            Some(properties),
            None,
            None,
        )
        .await
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

    fn get_profile_id() -> Option<String> {
        Some("rust_123123123".to_string())
    }

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
            .track(
                "test_event".to_string(),
                get_profile_id(),
                Some(properties),
                None,
                None,
            )
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
            .track(
                "test_event".to_string(),
                get_profile_id(),
                Some(properties),
                None,
                Some(&filter),
            )
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
        let response = tracker.revenue(get_profile_id(), 100, None).await?;

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

    #[tokio::test]
    async fn can_set_group_persists_group() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.disable();
        // Even though the tracker is disabled (send_request returns error),
        // the group ID should still be persisted.
        let _ = tracker
            .set_group("profile_123".to_string(), "org_acme".to_string())
            .await;
        let groups = tracker.groups.read().unwrap();
        assert!(groups.contains(&"org_acme".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn can_set_groups_persists_multiple() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.disable();
        let _ = tracker
            .set_groups(
                "profile_123".to_string(),
                vec!["org_acme".to_string(), "team_engineering".to_string()],
            )
            .await;
        let groups = tracker.groups.read().unwrap();
        assert!(groups.contains(&"org_acme".to_string()));
        assert!(groups.contains(&"team_engineering".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn can_clear_groups() -> anyhow::Result<()> {
        let tracker = Tracker::try_new_from_env()?.disable();
        let _ = tracker
            .set_group("profile_123".to_string(), "org_acme".to_string())
            .await;
        tracker.clear();
        let groups = tracker.groups.read().unwrap();
        assert!(groups.is_empty());
        Ok(())
    }
}
