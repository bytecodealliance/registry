use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RegistryInfo {
    url: String,
}

impl RegistryInfo {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}
