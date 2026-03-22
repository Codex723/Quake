#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

use crate::{
    errors::QuakeError,
    types::{
        InsuranceVertical, OracleParameter, PolicyState, TriggerConfig, TriggerDirection,
    },
    QuakeContract, QuakeContractClient,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Env, QuakeContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, QuakeContract);
    let client = QuakeContractClient::new(&env, &id);

    client
        .initialize(&Address::generate(&env), &30_u32, &2_u32)
        .unwrap();

    (env, client)
}

fn location(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn crop_trigger(confirmation_window: u32) -> TriggerConfig {
    TriggerConfig {
        parameter: OracleParameter::RainfallMm,
        threshold: 5000_i128, // 50.00 mm
        direction: TriggerDirection::Below,
        confirmation_window,
    }
}

fn create_basic_policy(
    env: &Env,
    client: &QuakeContractClient,
) -> (Address, crate::types::PolicyId) {
    let policyholder = Address::generate(env);
    let asset = Address::generate(env);
    let loc = location(env, 1);

    let id = client
        .create_policy(
            &policyholder,
            &InsuranceVertical::CropRainfall,
            &asset,
            &500_0000000_i128,
            &asset,
            &1_000_i128,
            &2_592_000_u32,
            &crop_trigger(2),
            &loc,
        )
        .unwrap();

    (policyholder, id)
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

#[test]
fn test_double_initialize_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let cid = env.register_contract(None, QuakeContract);
    let client = QuakeContractClient::new(&env, &cid);

    let admin = Address::generate(&env);
    client.initialize(&admin, &30_u32, &2_u32).unwrap();

    let err = client.initialize(&admin, &30_u32, &2_u32).unwrap_err();
    assert_eq!(err, QuakeError::AlreadyInitialized.into());
}

// ---------------------------------------------------------------------------
// register_oracle_source
// ---------------------------------------------------------------------------

#[test]
fn test_register_oracle_source() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    // Re-initialize with known admin
    let cid = env.register_contract(None, QuakeContract);
    let client2 = QuakeContractClient::new(&env, &cid);
    client2.initialize(&admin, &30_u32, &2_u32).unwrap();

    let source = Address::generate(&env);
    client2
        .register_oracle_source(&admin, &source, &InsuranceVertical::CropRainfall)
        .unwrap();

    let record = client2
        .get_oracle_source(&InsuranceVertical::CropRainfall, &source)
        .unwrap();

    assert_eq!(record.reliability_score, 50);
    assert_eq!(record.address, source);
}

#[test]
fn test_non_admin_cannot_register_oracle() {
    let (env, client) = setup();
    let attacker = Address::generate(&env);

    let err = client
        .register_oracle_source(
            &attacker,
            &Address::generate(&env),
            &InsuranceVertical::CropRainfall,
        )
        .unwrap_err();

    assert_eq!(err, QuakeError::Unauthorised.into());
}

// ---------------------------------------------------------------------------
// submit_oracle_reading
// ---------------------------------------------------------------------------

#[test]
fn test_submit_oracle_reading_stores_value() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let cid = env.register_contract(None, QuakeContract);
    let client2 = QuakeContractClient::new(&env, &cid);
    client2.initialize(&admin, &30_u32, &2_u32).unwrap();

    let oracle = Address::generate(&env);
    client2
        .register_oracle_source(&admin, &oracle, &InsuranceVertical::CropRainfall)
        .unwrap();

    let loc = location(&env, 5);
    env.ledger().with_mut(|l| l.sequence_number = 1000);

    client2
        .submit_oracle_reading(
            &oracle,
            &InsuranceVertical::CropRainfall,
            &loc,
            &3000_i128, // 30.00 mm — below drought threshold
            &1000_u32,
        )
        .unwrap();

    let reading = client2.get_accepted_reading(&InsuranceVertical::CropRainfall, &loc);
    assert_eq!(reading, Some(3000_i128));
}

#[test]
fn test_stale_reading_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let cid = env.register_contract(None, QuakeContract);
    let client2 = QuakeContractClient::new(&env, &cid);
    client2.initialize(&admin, &30_u32, &2_u32).unwrap();

    let oracle = Address::generate(&env);
    client2
        .register_oracle_source(&admin, &oracle, &InsuranceVertical::CropRainfall)
        .unwrap();

    // Advance to ledger 2000, submit reading timestamped at ledger 1 (stale)
    env.ledger().with_mut(|l| l.sequence_number = 2000);

    let err = client2
        .submit_oracle_reading(
            &oracle,
            &InsuranceVertical::CropRainfall,
            &location(&env, 6),
            &3000_i128,
            &1_u32, // way too old
        )
        .unwrap_err();

    assert_eq!(err, QuakeError::OracleReadingStale.into());
}

#[test]
fn test_unregistered_oracle_cannot_submit() {
    let (env, client) = setup();

    let err = client
        .submit_oracle_reading(
            &Address::generate(&env),
            &InsuranceVertical::CropRainfall,
            &location(&env, 7),
            &3000_i128,
            &0_u32,
        )
        .unwrap_err();

    assert_eq!(err, QuakeError::OracleNotRegistered.into());
}

// ---------------------------------------------------------------------------
// create_policy
// ---------------------------------------------------------------------------

#[test]
fn test_create_policy_stores_record() {
    let (env, client) = setup();
    let (policyholder, id) = create_basic_policy(&env, &client);

    let record = client.get_policy(&id).unwrap();
    assert_eq!(record.policyholder, policyholder);
    assert_eq!(record.payout_amount, 500_0000000);
    assert!(matches!(record.state, PolicyState::Active));
    assert_eq!(record.confirmation_count, 0);
    assert!(record.trigger_ledger.is_none());
}

#[test]
fn test_create_policy_rejects_zero_payout() {
    let (env, client) = setup();
    let asset = Address::generate(&env);

    let err = client
        .create_policy(
            &Address::generate(&env),
            &InsuranceVertical::CropRainfall,
            &asset,
            &0_i128,
            &asset,
            &1_000_i128,
            &100_u32,
            &crop_trigger(2),
            &location(&env, 8),
        )
        .unwrap_err();

    assert_eq!(err, QuakeError::InvalidPayoutAmount.into());
}

#[test]
fn test_create_policy_rejects_zero_premium() {
    let (env, client) = setup();
    let asset = Address::generate(&env);

    let err = client
        .create_policy(
            &Address::generate(&env),
            &InsuranceVertical::CropRainfall,
            &asset,
            &100_0000000_i128,
            &asset,
            &0_i128,
            &100_u32,
            &crop_trigger(2),
            &location(&env, 9),
        )
        .unwrap_err();

    assert_eq!(err, QuakeError::InvalidPremiumRate.into());
}

#[test]
fn test_create_policy_rejects_zero_confirmation_window() {
    let (env, client) = setup();
    let asset = Address::generate(&env);

    let err = client
        .create_policy(
            &Address::generate(&env),
            &InsuranceVertical::CropRainfall,
            &asset,
            &100_0000000_i128,
            &asset,
            &1_000_i128,
            &100_u32,
            &crop_trigger(0), // window = 0, invalid
            &location(&env, 10),
        )
        .unwrap_err();

    assert_eq!(err, QuakeError::InvalidTriggerConfig.into());
}

#[test]
fn test_policy_end_ledger_set_correctly() {
    let (env, client) = setup();
    env.ledger().with_mut(|l| l.sequence_number = 500);

    let asset = Address::generate(&env);
    let id = client
        .create_policy(
            &Address::generate(&env),
            &InsuranceVertical::CropRainfall,
            &asset,
            &100_0000000_i128,
            &asset,
            &1_000_i128,
            &1_000_u32, // duration
            &crop_trigger(2),
            &location(&env, 11),
        )
        .unwrap();

    let record = client.get_policy(&id).unwrap();
    assert_eq!(record.end_ledger, 1500); // 500 + 1000
}

// ---------------------------------------------------------------------------
// cancel_policy
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_policy_sets_state() {
    let (env, client) = setup();
    let (policyholder, id) = create_basic_policy(&env, &client);

    client.cancel_policy(&id, &policyholder).unwrap();

    let record = client.get_policy(&id).unwrap();
    assert!(matches!(record.state, PolicyState::Cancelled));
}

#[test]
fn test_non_policyholder_cannot_cancel() {
    let (env, client) = setup();
    let (_, id) = create_basic_policy(&env, &client);
    let attacker = Address::generate(&env);

    let err = client.cancel_policy(&id, &attacker).unwrap_err();
    assert_eq!(err, QuakeError::Unauthorised.into());
}

#[test]
fn test_cancelled_policy_cannot_be_cancelled_again() {
    let (env, client) = setup();
    let (policyholder, id) = create_basic_policy(&env, &client);

    client.cancel_policy(&id, &policyholder).unwrap();

    let err = client.cancel_policy(&id, &policyholder).unwrap_err();
    assert_eq!(err, QuakeError::PolicyNotActive.into());
}

// ---------------------------------------------------------------------------
// is_trigger_active
// ---------------------------------------------------------------------------

#[test]
fn test_is_trigger_active_below_threshold() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let cid = env.register_contract(None, QuakeContract);
    let client2 = QuakeContractClient::new(&env, &cid);
    client2.initialize(&admin, &30_u32, &2_u32).unwrap();

    let oracle = Address::generate(&env);
    client2
        .register_oracle_source(&admin, &oracle, &InsuranceVertical::CropRainfall)
        .unwrap();

    let loc = location(&env, 12);
    env.ledger().with_mut(|l| l.sequence_number = 100);

    // Submit reading of 30mm (3000), threshold is 50mm (5000)
    client2
        .submit_oracle_reading(
            &oracle,
            &InsuranceVertical::CropRainfall,
            &loc,
            &3000_i128,
            &100_u32,
        )
        .unwrap();

    // Below threshold = trigger active
    assert!(client2.is_trigger_active(
        &InsuranceVertical::CropRainfall,
        &loc,
        &5000_i128,
        &TriggerDirection::Below,
    ));

    // Above threshold = not triggered
    assert!(!client2.is_trigger_active(
        &InsuranceVertical::CropRainfall,
        &loc,
        &5000_i128,
        &TriggerDirection::Above,
    ));
}

#[test]
fn test_is_trigger_active_no_reading_returns_false() {
    let (env, client) = setup();
    let loc = location(&env, 13);

    assert!(!client.is_trigger_active(
        &InsuranceVertical::CropRainfall,
        &loc,
        &5000_i128,
        &TriggerDirection::Below,
    ));
}
