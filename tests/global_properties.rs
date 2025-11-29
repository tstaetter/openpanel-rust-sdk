use openpanel_sdk::sdk::Tracker;
use std::collections::HashMap;

#[tokio::test]
async fn can_track_event() -> anyhow::Result<()> {
    let global_properties = HashMap::from([("global".to_string(), "property".to_string())]);
    let local_properties = HashMap::from([("local".to_string(), "property".to_string())]);
    let tracker = Tracker::try_new_from_env()?
        .with_default_headers()?
        .with_global_properties(global_properties);
    let response = tracker
        .track("test_event".to_string(), Some(local_properties))
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}
