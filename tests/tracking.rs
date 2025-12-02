use openpanel_sdk::sdk::{user, Tracker};
use std::collections::HashMap;

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
async fn can_filter_event() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let mut properties = HashMap::new();

    properties.insert("name".to_string(), "rust".to_string());

    let response = tracker
        .track("test_event".to_string(), Some(properties), None)
        .await;

    assert!(response.is_err());

    Ok(())
}

#[tokio::test]
async fn can_filter_track_event() -> anyhow::Result<()> {
    let filter = |properties: HashMap<String, String>| properties.contains_key("not-existing");
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
