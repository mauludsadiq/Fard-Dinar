use anyhow::{anyhow, Result};
use fd_core::{Event, LedgerState, Receipt, RegistryState};
use serde::de::DeserializeOwned;

pub struct Client {
    base_url: String,
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn submit_event(&self, event: &Event) -> Result<serde_json::Value> {
        let body = serde_json::to_string(event)?;
        let resp = ureq::post(&format!("{}/v1/events", self.base_url))
            .set("content-type", "application/json")
            .send_string(&body)
            .map_err(|e| anyhow!("submit_event failed: {}", e))?;
        let text = resp
            .into_string()
            .map_err(|e| anyhow!("submit_event read failed: {}", e))?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn get_registry(&self) -> Result<RegistryState> {
        self.get_json("/v1/registry")
    }

    pub fn get_state(&self) -> Result<LedgerState> {
        self.get_json("/v1/state")
    }

    pub fn get_receipt(&self, run_id: &str) -> Result<Receipt> {
        self.get_json(&format!("/v1/receipts/{}", run_id))
    }

    pub fn get_object(&self, hash: &str) -> Result<serde_json::Value> {
        self.get_json(&format!("/v1/objects/{}", hash))
    }

    pub fn get_info(&self) -> Result<serde_json::Value> {
        self.get_json("/v1/info")
    }

    fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = ureq::get(&format!("{}{}", self.base_url, path))
            .call()
            .map_err(|e| anyhow!("GET {} failed: {}", path, e))?;
        let text = resp
            .into_string()
            .map_err(|e| anyhow!("GET {} read failed: {}", path, e))?;
        Ok(serde_json::from_str(&text)?)
    }
}
