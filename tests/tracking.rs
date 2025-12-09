use openpanel_sdk::sdk::{user, Tracker};
use std::collections::HashMap;

fn get_profile_id() -> Option<String> {
    Some("rust_123123123".to_string())
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
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_apply_no_filter() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let mut properties = HashMap::new();

    properties.insert("name".to_string(), "rust".to_string());

    let response = tracker
        .track(
            "test_event".to_string(),
            get_profile_id(),
            Some(properties),
            None,
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_apply_filter_track_event() -> anyhow::Result<()> {
    let filter = |properties: HashMap<String, String>| properties.contains_key("name");
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let mut properties = HashMap::new();

    properties.insert("name".to_string(), "rust".to_string());

    let response = tracker
        .track(
            "test_event".to_string(),
            get_profile_id(),
            Some(properties),
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
        profile_id: get_profile_id().unwrap(),
        email: "rust@test.com".to_string(),
        first_name: "rust".to_string(),
        last_name: "tester".to_string(),
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
    let properties = HashMap::from([("currency".to_string(), "EUR".to_string())]);
    let response = tracker
        .revenue(get_profile_id(), 100, Some(properties))
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}
