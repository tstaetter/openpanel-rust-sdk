use openpanel_sdk::sdk::{Tracker, group::Group};
use std::collections::HashMap;

fn get_profile_id() -> String {
    "rust_123123123".to_string()
}

#[tokio::test]
async fn can_upsert_group() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let group = Group {
        id: "org_acme".to_string(),
        group_type: "company".to_string(),
        name: "Acme Inc".to_string(),
        properties: HashMap::from([
            ("plan".to_string(), "enterprise".to_string()),
            ("seats".to_string(), "25".to_string()),
        ]),
    };

    let response = tracker.upsert_group(group).await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_upsert_group_without_properties() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let group = Group {
        id: "org_minimal".to_string(),
        group_type: "workspace".to_string(),
        name: "Minimal Workspace".to_string(),
        properties: HashMap::new(),
    };

    let response = tracker.upsert_group(group).await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_set_group() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let response = tracker
        .set_group(get_profile_id(), "org_acme".to_string())
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_set_multiple_groups() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let response = tracker
        .set_groups(
            get_profile_id(),
            vec!["org_acme".to_string(), "team_engineering".to_string()],
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_track_with_per_event_groups() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let mut properties = HashMap::new();

    properties.insert("filename".to_string(), "q4-report.pdf".to_string());

    let response = tracker
        .track(
            "file_shared".to_string(),
            Some(get_profile_id()),
            Some(properties),
            Some(vec!["org_acme".to_string(), "org_partner".to_string()]),
            None,
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_track_with_persistent_groups() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;

    // Set a persistent group first
    let _ = tracker
        .set_group(get_profile_id(), "org_acme".to_string())
        .await?;

    // Now track an event — the persistent group should be included
    let mut properties = HashMap::new();
    properties.insert("page".to_string(), "dashboard".to_string());

    let response = tracker
        .track(
            "page_viewed".to_string(),
            Some(get_profile_id()),
            Some(properties),
            None,
            None,
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn can_clear_groups_on_logout() -> anyhow::Result<()> {
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;

    // Set groups for the current user
    let _ = tracker
        .set_groups(
            get_profile_id(),
            vec!["org_acme".to_string(), "team_engineering".to_string()],
        )
        .await?;

    // Clear on logout
    tracker.clear();

    // After clear, track should not include persistent groups
    let mut properties = HashMap::new();
    properties.insert("action".to_string(), "post_logout_interaction".to_string());

    let response = tracker
        .track(
            "generic_event".to_string(),
            Some(get_profile_id()),
            Some(properties),
            None,
            None,
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}
