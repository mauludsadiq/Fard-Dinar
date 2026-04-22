use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use fd_core::{
    deposit_signing_payload, transfer_signing_payload, DepositAttestation, Event, LedgerState,
    Receipt, RegistryState, TransferIntent,
};
use serde::de::DeserializeOwned;

pub struct Client {
    base_url: String,
}

pub struct Wallet {
    signing_key: SigningKey,
}

impl Wallet {
    pub fn from_secret_hex(secret_hex: &str) -> Result<Self> {
        let bytes = hex::decode(secret_hex).context("invalid secret key hex")?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("secret key must be 32 bytes"))?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&arr),
        })
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }

    pub fn build_signed_transfer(&self, to: &str, amount: u64, nonce: u64) -> TransferIntent {
        let mut tx = TransferIntent {
            kind: "transfer".to_string(),
            from_key: self.public_key_hex(),
            to_key: to.to_string(),
            amount,
            nonce,
            signature: String::new(),
        };
        let sig = self.signing_key.sign(&transfer_signing_payload(&tx));
        tx.signature = hex::encode(sig.to_bytes());
        tx
    }

    pub fn build_signed_deposit(
        &self,
        beneficiary: &str,
        usd_cents: u64,
        external_ref: &str,
        timestamp: u64,
    ) -> DepositAttestation {
        let mut dep = DepositAttestation {
            kind: "deposit".to_string(),
            oracle_id: self.public_key_hex(),
            usd_cents,
            beneficiary: beneficiary.to_string(),
            external_ref: external_ref.to_string(),
            timestamp,
            signature: String::new(),
        };
        let sig = self.signing_key.sign(&deposit_signing_payload(&dep));
        dep.signature = hex::encode(sig.to_bytes());
        dep
    }

    pub fn build_signed_transfer_event(&self, to: &str, amount: u64, nonce: u64) -> Event {
        Event::Transfer(self.build_signed_transfer(to, amount, nonce))
    }

    pub fn build_signed_deposit_event(
        &self,
        beneficiary: &str,
        usd_cents: u64,
        external_ref: &str,
        timestamp: u64,
    ) -> Event {
        Event::Deposit(self.build_signed_deposit(
            beneficiary,
            usd_cents,
            external_ref,
            timestamp,
        ))
    }
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

    
    pub fn submit_signed_transfer(
        &self,
        wallet: &Wallet,
        to: &str,
        amount: u64,
        nonce: u64,
    ) -> Result<serde_json::Value> {
        let evt = wallet.build_signed_transfer_event(to, amount, nonce);
        self.submit_event(&evt)
    }

    pub fn submit_signed_deposit(
        &self,
        wallet: &Wallet,
        beneficiary: &str,
        usd_cents: u64,
        external_ref: &str,
        timestamp: u64,
    ) -> Result<serde_json::Value> {
        let evt = wallet.build_signed_deposit_event(
            beneficiary,
            usd_cents,
            external_ref,
            timestamp,
        );
        self.submit_event(&evt)
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
