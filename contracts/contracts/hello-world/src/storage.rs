use soroban_sdk::{Address, Bytes, BytesN, Env};

use crate::errors::QuakeError;
use crate::types::{
    ContractConfig, DataKey, InsuranceVertical, OracleConfig, OracleSource,
    PolicyId, PolicyRecord, TriggerConfig,
};

const POLICY_TTL: u32 = 6_307_200;
const CONFIG_TTL: u32 = 6_307_200;

// ---------------------------------------------------------------------------
// Policy helpers
// ---------------------------------------------------------------------------

pub fn save_policy(env: &Env, id: &PolicyId, record: &PolicyRecord) {
    let key = DataKey::Policy(id.clone());
    env.storage().persistent().set(&key, record);
    env.storage().persistent().extend_ttl(&key, POLICY_TTL, POLICY_TTL);
}

pub fn load_policy(env: &Env, id: &PolicyId) -> Result<PolicyRecord, QuakeError> {
    env.storage()
        .persistent()
        .get::<DataKey, PolicyRecord>(&DataKey::Policy(id.clone()))
        .ok_or(QuakeError::PolicyNotFound)
}

pub fn policy_exists(env: &Env, id: &PolicyId) -> bool {
    env.storage().persistent().has(&DataKey::Policy(id.clone()))
}

/// Deterministic PolicyId from key minting inputs.
pub fn derive_policy_id(
    env: &Env,
    policyholder: &Address,
    vertical: &InsuranceVertical,
    payout_amount: i128,
    due_ledger: u32,
) -> PolicyId {
    let mut buf = Bytes::new(env);
    buf.append(&policyholder.to_xdr(env));
    buf.append(&vertical.to_xdr(env));
    buf.append(&Bytes::from_slice(env, &payout_amount.to_be_bytes()));
    buf.append(&Bytes::from_slice(env, &due_ledger.to_be_bytes()));
    buf.append(&Bytes::from_slice(
        env,
        &env.ledger().sequence().to_be_bytes(),
    ));
    env.crypto().sha256(&buf)
}

// ---------------------------------------------------------------------------
// Oracle source helpers
// ---------------------------------------------------------------------------

pub fn save_oracle_source(env: &Env, source: &OracleSource) {
    let key = DataKey::OracleSource(source.vertical.clone(), source.address.clone());
    env.storage().persistent().set(&key, source);
    env.storage().persistent().extend_ttl(&key, CONFIG_TTL, CONFIG_TTL);
}

pub fn load_oracle_source(
    env: &Env,
    vertical: &InsuranceVertical,
    address: &Address,
) -> Result<OracleSource, QuakeError> {
    env.storage()
        .persistent()
        .get::<DataKey, OracleSource>(&DataKey::OracleSource(
            vertical.clone(),
            address.clone(),
        ))
        .ok_or(QuakeError::OracleNotRegistered)
}

pub fn oracle_source_exists(
    env: &Env,
    vertical: &InsuranceVertical,
    address: &Address,
) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::OracleSource(vertical.clone(), address.clone()))
}

// ---------------------------------------------------------------------------
// Accepted oracle reading helpers
// ---------------------------------------------------------------------------

pub fn save_accepted_reading(
    env: &Env,
    vertical: &InsuranceVertical,
    location_hash: &BytesN<32>,
    value: i128,
) {
    let key = DataKey::AcceptedReading(vertical.clone(), location_hash.clone());
    env.storage().persistent().set(&key, &value);
    env.storage().persistent().extend_ttl(&key, CONFIG_TTL, CONFIG_TTL);
}

pub fn load_accepted_reading(
    env: &Env,
    vertical: &InsuranceVertical,
    location_hash: &BytesN<32>,
) -> Option<i128> {
    env.storage()
        .persistent()
        .get::<DataKey, i128>(&DataKey::AcceptedReading(
            vertical.clone(),
            location_hash.clone(),
        ))
}

// ---------------------------------------------------------------------------
// Oracle config helpers
// ---------------------------------------------------------------------------

pub fn save_oracle_config(env: &Env, vertical: &InsuranceVertical, config: &OracleConfig) {
    let key = DataKey::OracleConfig(vertical.clone());
    env.storage().persistent().set(&key, config);
    env.storage().persistent().extend_ttl(&key, CONFIG_TTL, CONFIG_TTL);
}

pub fn load_oracle_config(env: &Env, vertical: &InsuranceVertical) -> OracleConfig {
    env.storage()
        .persistent()
        .get::<DataKey, OracleConfig>(&DataKey::OracleConfig(vertical.clone()))
        .unwrap_or(OracleConfig {
            required: 3,
            tolerance_pct: 10,
            staleness_ttl: 720,
        })
}

// ---------------------------------------------------------------------------
// Contract config helpers
// ---------------------------------------------------------------------------

pub fn save_config(env: &Env, config: &ContractConfig) {
    env.storage().instance().set(&DataKey::Config, config);
}

pub fn load_config(env: &Env) -> ContractConfig {
    env.storage()
        .instance()
        .get::<DataKey, ContractConfig>(&DataKey::Config)
        .expect("Quake: contract not initialized")
}

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Config)
}
