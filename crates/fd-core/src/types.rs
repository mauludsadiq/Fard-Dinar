use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type PublicKeyHex = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub balance: u64,
    pub next_nonce: u64,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            balance: 0,
            next_nonce: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewardConfig {
    pub user_p2p_bps: u32,
    pub user_spend_bps: u32,
    pub vendor_spend_bps: u32,
    pub treasury_account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerState {
    pub accounts: BTreeMap<PublicKeyHex, Account>,
    pub consumed_deposits: BTreeSet<String>,
    pub merchant_registry_hash: String,
    pub oracle_set_hash: String,
    pub reward_config: RewardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferIntent {
    pub kind: String,
    #[serde(rename = "from")]
    pub from_key: PublicKeyHex,
    #[serde(rename = "to")]
    pub to_key: PublicKeyHex,
    pub amount: u64,
    pub nonce: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DepositAttestation {
    pub kind: String,
    pub oracle_id: PublicKeyHex,
    pub usd_cents: u64,
    pub beneficiary: PublicKeyHex,
    pub external_ref: String,
    pub timestamp: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerchantRegistrySnapshot {
    pub version: String,
    pub merchants: Vec<PublicKeyHex>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OracleSetSnapshot {
    pub version: String,
    pub oracles: Vec<PublicKeyHex>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConfiguration {
    pub kind: String,
    pub accounts: BTreeMap<PublicKeyHex, Account>,
    pub consumed_deposits: Vec<String>,
    pub merchant_registry_hash: String,
    pub oracle_set_hash: String,
    pub reward_config: RewardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Event {
    Transfer(TransferIntent),
    Deposit(DepositAttestation),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryEntry {
    pub effect_kind: String,
    pub conflict_key: String,
    pub event_hash: String,
    pub run_id: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryState {
    pub entries: BTreeMap<String, RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferEffects {
    pub amount: u64,
    pub user_reward: u64,
    pub vendor_reward: u64,
    pub is_merchant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub run_id: String,
    pub program_hash: String,
    pub input_hash: String,
    pub pre_state_hash: String,
    pub post_state_hash: String,
    pub trace_hash: String,
    pub transfer_effects: Option<TransferEffects>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionResult {
    pub state: LedgerState,
    pub receipt: Receipt,
}

impl LedgerState {
    pub fn account_or_default(&self, key: &str) -> Account {
        self.accounts.get(key).cloned().unwrap_or_default()
    }

    pub fn materialize_account_mut(&mut self, key: &str) -> &mut Account {
        self.accounts.entry(key.to_string()).or_default()
    }
}

impl From<GenesisConfiguration> for LedgerState {
    fn from(value: GenesisConfiguration) -> Self {
        Self {
            accounts: value.accounts,
            consumed_deposits: value.consumed_deposits.into_iter().collect(),
            merchant_registry_hash: value.merchant_registry_hash,
            oracle_set_hash: value.oracle_set_hash,
            reward_config: value.reward_config,
        }
    }
}
