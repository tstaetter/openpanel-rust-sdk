use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifyUser {
    pub profile_id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub properties: HashMap<String, String>,
}
