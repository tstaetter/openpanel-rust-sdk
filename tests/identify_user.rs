use openpanel_sdk::sdk::{user, Tracker};
use std::collections::HashMap;

struct Address {
    pub street: String,
    pub city: String,
    pub zip: String,
}

struct AppUser {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub address: Address,
}

impl From<Address> for HashMap<String, String> {
    fn from(address: Address) -> Self {
        let mut properties = HashMap::new();

        properties.insert("street".to_string(), address.street);
        properties.insert("city".to_string(), address.city);
        properties.insert("zip".to_string(), address.zip);

        properties
    }
}

impl From<AppUser> for user::IdentifyUser {
    fn from(app_user: AppUser) -> Self {
        Self {
            profile_id: app_user.id,
            email: app_user.email,
            first_name: app_user.first_name,
            last_name: app_user.last_name,
            properties: app_user.address.into(),
        }
    }
}

#[tokio::test]
async fn can_identify_user() -> anyhow::Result<()> {
    let user = AppUser {
        id: "test_profile_id".to_string(),
        email: "rust@test.com".to_string(),
        first_name: "rust".to_string(),
        last_name: "tester".to_string(),
        address: Address {
            street: "bondstreet 1a".to_string(),
            city: "London".to_string(),
            zip: "12345".to_string(),
        },
    };
    let tracker = Tracker::try_new_from_env()?.with_default_headers()?;
    let response = tracker.identify(user.into()).await?;

    assert_eq!(response.status(), 200);

    Ok(())
}
