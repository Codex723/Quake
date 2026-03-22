#![no_std]

mod errors;
mod storage;
mod types;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol};

use errors::QuakeError;
use storage::{
    derive_policy_id, is_initialized, load_accepted_reading, load_config,
    load_oracle_config, load_oracle_source, load_policy, oracle_source_exists,
    save_accepted_reading, save_config, save_oracle_source, save_policy,
};
use types::{
    ContractConfig, InsuranceVertical, OracleConfig, OracleSource, PolicyId,
    PolicyRecord, PolicyState, TriggerConfig, TriggerDirection,
    EVENT_ORACLE_CONSENSUS, EVENT_ORACLE_FAILED, EVENT_ORACLE_SUBMITTED,
    EVENT_POLICY_CANCELLED, EVENT_POLICY_CREATED, EVENT_POLICY_TRIGGERED,
    EVENT_PREMIUM_DEDUCTED,
};

#[contract]
pub struct QuakeContract;

#[contractimpl]
impl QuakeContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initialize the Quake contract. Must be called once after deployment.
    ///
    /// # Errors
    /// - `QuakeError::AlreadyInitialized` — called more than once.
    pub fn initialize(
        env: Env,
        admin: Address,
        min_reliability_score: u32,
        default_confirmation_window: u32,
    ) -> Result<(), QuakeError> {
        if is_initialized(&env) {
            return Err(QuakeError::AlreadyInitialized);
        }
        admin.require_auth();

        save_config(
            &env,
            &ContractConfig {
                admin,
                min_reliability_score,
                default_confirmation_window,
                min_solvency_ratio_bps: 1000,
            },
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Oracle management (admin)
    // -----------------------------------------------------------------------

    /// Register a new oracle source for a given vertical.
    ///
    /// Only the admin may register sources. Newly registered sources start
    /// with a reliability_score of 50 (neutral — must earn trust over time).
    ///
    /// # Errors
    /// - `QuakeError::Unauthorised` — caller is not the admin.
    pub fn register_oracle_source(
        env: Env,
        admin: Address,
        source_address: Address,
        vertical: InsuranceVertical,
    ) -> Result<(), QuakeError> {
        admin.require_auth();

        let config = load_config(&env);
        if config.admin != admin {
            return Err(QuakeError::Unauthorised);
        }

        let source = OracleSource {
            address: source_address,
            vertical,
            reliability_score: 50,
            last_submission_ledger: 0,
        };
        save_oracle_source(&env, &source);
        Ok(())
    }

    /// Configure oracle consensus parameters for a specific vertical.
    pub fn set_oracle_config(
        env: Env,
        admin: Address,
        vertical: InsuranceVertical,
        required: u32,
        tolerance_pct: u32,
        staleness_ttl: u32,
    ) -> Result<(), QuakeError> {
        admin.require_auth();

        let config = load_config(&env);
        if config.admin != admin {
            return Err(QuakeError::Unauthorised);
        }

        storage::save_oracle_config(
            &env,
            &vertical,
            &OracleConfig {
                required,
                tolerance_pct,
                staleness_ttl,
            },
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Oracle reading submission
    // -----------------------------------------------------------------------

    /// Submit a new parameter reading from a registered oracle source.
    ///
    /// Readings are stored per source. The contract does not immediately
    /// accept the reading — consensus evaluation happens in `evaluate_triggers`.
    ///
    /// # Errors
    /// - `QuakeError::OracleNotRegistered` — source not in registry.
    /// - `QuakeError::OracleReadingStale`  — timestamp older than staleness_ttl.
    ///
    /// # Events
    /// Emits `OracleReadingSubmitted` with `(oracle, vertical, location_hash, value)`.
    pub fn submit_oracle_reading(
        env: Env,
        oracle: Address,
        vertical: InsuranceVertical,
        location_hash: BytesN<32>,
        value: i128,
        ledger_timestamp: u32,
    ) -> Result<(), QuakeError> {
        oracle.require_auth();

        let mut source = load_oracle_source(&env, &vertical, &oracle)?;
        let oracle_cfg = load_oracle_config(&env, &vertical);

        // Reject stale readings
        let current = env.ledger().sequence();
        if current.saturating_sub(ledger_timestamp) > oracle_cfg.staleness_ttl {
            return Err(QuakeError::OracleReadingStale);
        }

        source.last_submission_ledger = current;
        save_oracle_source(&env, &source);

        // Store individual reading keyed by (vertical, location, oracle)
        // In production this would go into a temporary readings buffer;
        // for the starter we store the accepted reading directly when a
        // single source submits, and the full N-of-M logic is a stub.
        // See issue #8 for full consensus implementation.
        save_accepted_reading(&env, &vertical, &location_hash, value);

        env.events().publish(
            (Symbol::new(&env, EVENT_ORACLE_SUBMITTED),),
            (oracle, vertical, location_hash, value),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Policy lifecycle
    // -----------------------------------------------------------------------

    /// Create a new parametric insurance policy.
    ///
    /// # Errors
    /// - `QuakeError::InvalidPayoutAmount`  — payout_amount ≤ 0.
    /// - `QuakeError::InvalidPremiumRate`   — premium_rate ≤ 0.
    /// - `QuakeError::InvalidTriggerConfig` — confirmation_window is 0.
    /// - `QuakeError::Unauthorised`         — policyholder did not sign.
    ///
    /// # Events
    /// Emits `PolicyCreated` with `(policy_id, policyholder, vertical, payout_amount)`.
    pub fn create_policy(
        env: Env,
        policyholder: Address,
        vertical: InsuranceVertical,
        coverage_asset: Address,
        payout_amount: i128,
        premium_asset: Address,
        premium_rate: i128,
        duration_ledgers: u32,
        trigger_config: TriggerConfig,
        location_hash: BytesN<32>,
    ) -> Result<PolicyId, QuakeError> {
        if payout_amount <= 0 {
            return Err(QuakeError::InvalidPayoutAmount);
        }
        if premium_rate <= 0 {
            return Err(QuakeError::InvalidPremiumRate);
        }
        if trigger_config.confirmation_window == 0 {
            return Err(QuakeError::InvalidTriggerConfig);
        }

        policyholder.require_auth();

        let current = env.ledger().sequence();
        let end_ledger = current
            .checked_add(duration_ledgers)
            .expect("ledger overflow");

        let id = derive_policy_id(&env, &policyholder, &vertical, payout_amount, end_ledger);

        let record = PolicyRecord {
            policyholder: policyholder.clone(),
            vertical: vertical.clone(),
            coverage_asset,
            payout_amount,
            premium_asset,
            premium_rate,
            start_ledger: current,
            end_ledger,
            trigger_config,
            location_hash,
            state: PolicyState::Active,
            trigger_ledger: None,
            confirmation_count: 0,
        };

        save_policy(&env, &id, &record);

        env.events().publish(
            (Symbol::new(&env, EVENT_POLICY_CREATED),),
            (id.clone(), policyholder, vertical, payout_amount),
        );

        Ok(id)
    }

    /// Cancel an active policy and stop premium deductions.
    ///
    /// Cannot cancel a policy that is `TriggeredPending` or already `Paid`.
    ///
    /// # Errors
    /// - `QuakeError::PolicyNotFound`  — unknown id.
    /// - `QuakeError::Unauthorised`    — caller is not the policyholder.
    /// - `QuakeError::PolicyNotActive` — policy is in a non-cancellable state.
    ///
    /// # Events
    /// Emits `PolicyCancelled`.
    pub fn cancel_policy(
        env: Env,
        policy_id: PolicyId,
        policyholder: Address,
    ) -> Result<(), QuakeError> {
        policyholder.require_auth();

        let mut record = load_policy(&env, &policy_id)?;

        if record.policyholder != policyholder {
            return Err(QuakeError::Unauthorised);
        }
        if !matches!(record.state, PolicyState::Active | PolicyState::Lapsed) {
            return Err(QuakeError::PolicyNotActive);
        }

        record.state = PolicyState::Cancelled;
        save_policy(&env, &policy_id, &record);

        env.events().publish(
            (Symbol::new(&env, EVENT_POLICY_CANCELLED),),
            (policy_id, policyholder),
        );

        Ok(())
    }

    /// Deduct the configured premium from the policyholder's wallet.
    ///
    /// Permissionless — any caller can trigger a deduction, enabling
    /// keeper bots to maintain the premium stream.
    ///
    /// If three consecutive deductions fail due to insufficient balance,
    /// the policy is marked `Lapsed`.
    ///
    /// # Events
    /// Emits `PremiumDeducted` or `PolicyLapsed`.
    pub fn deduct_premium(env: Env, policy_id: PolicyId) -> Result<(), QuakeError> {
        let record = load_policy(&env, &policy_id)?;

        if !matches!(record.state, PolicyState::Active) {
            return Err(QuakeError::PolicyNotActive);
        }

        // Premium transfer logic (token interface call) tracked in issue #16.
        // For the starter we emit the event to show the contract structure.
        env.events().publish(
            (Symbol::new(&env, EVENT_PREMIUM_DEDUCTED),),
            (policy_id, record.premium_rate),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // View functions
    // -----------------------------------------------------------------------

    /// Retrieve the full PolicyRecord. No auth required.
    pub fn get_policy(env: Env, policy_id: PolicyId) -> Result<PolicyRecord, QuakeError> {
        load_policy(&env, &policy_id)
    }

    /// Retrieve the latest accepted oracle reading for a vertical and location.
    pub fn get_accepted_reading(
        env: Env,
        vertical: InsuranceVertical,
        location_hash: BytesN<32>,
    ) -> Option<i128> {
        load_accepted_reading(&env, &vertical, &location_hash)
    }

    /// Check whether a trigger condition is currently active for a location.
    ///
    /// Returns true if the latest accepted reading for the vertical and
    /// location crosses the threshold in the specified direction.
    pub fn is_trigger_active(
        env: Env,
        vertical: InsuranceVertical,
        location_hash: BytesN<32>,
        threshold: i128,
        direction: TriggerDirection,
    ) -> bool {
        match load_accepted_reading(&env, &vertical, &location_hash) {
            Some(reading) => match direction {
                TriggerDirection::Above => reading > threshold,
                TriggerDirection::Below => reading < threshold,
            },
            None => false,
        }
    }

    /// Get the oracle source record for a specific source address and vertical.
    pub fn get_oracle_source(
        env: Env,
        vertical: InsuranceVertical,
        source: Address,
    ) -> Result<OracleSource, QuakeError> {
        load_oracle_source(&env, &vertical, &source)
    }

    // -----------------------------------------------------------------------
    // Stubs — tracked in GitHub issues
    // -----------------------------------------------------------------------
    // evaluate_triggers   — issue #10 (full N-of-M evaluation)
    // confirm_payout      — issue #12
    // stake_reserve       — issue #15
    // unstake_reserve     — issue #15
    // claim_yield         — issue #33
    // mark_defaulted      — issue #13 (for ReservePool absorb)
    // upgrade             — issue #39
}
