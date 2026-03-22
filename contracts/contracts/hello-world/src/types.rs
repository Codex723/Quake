use soroban_sdk::{contracttype, Address, BytesN, String};

/// Unique identifier for a policy — SHA-256 of key creation inputs.
pub type PolicyId = BytesN<32>;

/// Unique identifier for an oracle reading.
pub type OracleReadingId = BytesN<32>;

// ---------------------------------------------------------------------------
// Policy record
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRecord {
    /// Wallet that owns and pays premiums for this policy.
    pub policyholder: Address,
    /// Which insurance vertical this policy covers.
    pub vertical: InsuranceVertical,
    /// Asset used for paying out claims (e.g. USDC contract address).
    pub coverage_asset: Address,
    /// Full payout amount on a confirmed trigger (7-decimal precision).
    pub payout_amount: i128,
    /// Asset policyholder pays premiums in.
    pub premium_asset: Address,
    /// Premium deducted per ledger (7-decimal precision).
    pub premium_rate: i128,
    /// Ledger at which coverage begins.
    pub start_ledger: u32,
    /// Ledger at which coverage expires without a trigger.
    pub end_ledger: u32,
    /// What parameter is monitored, at what threshold, and how long it must
    /// persist before a payout is confirmed.
    pub trigger_config: TriggerConfig,
    /// SHA-256 of the geographic location string (lat/lon or flight number).
    /// Used to match oracle readings to this policy.
    pub location_hash: BytesN<32>,
    /// Current lifecycle state.
    pub state: PolicyState,
    /// Ledger at which the trigger condition was first detected.
    pub trigger_ledger: Option<u32>,
    /// How many consecutive oracle updates have confirmed the trigger.
    pub confirmation_count: u32,
}

// ---------------------------------------------------------------------------
// Trigger configuration
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TriggerConfig {
    /// Which measured parameter this policy monitors.
    pub parameter: OracleParameter,
    /// The threshold value (scaled integer — see oracle README for units).
    pub threshold: i128,
    /// Whether the condition fires above or below the threshold.
    pub direction: TriggerDirection,
    /// Number of consecutive confirming oracle updates required before
    /// a payout is finalised.  Prevents false triggers from data spikes.
    pub confirmation_window: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleParameter {
    /// Monthly or weekly rainfall in mm × 100.
    RainfallMm,
    /// Flight delay in minutes × 1.
    FlightDelayMinutes,
    /// Wind speed in km/h × 10.
    WindSpeedKmh,
    /// Earthquake magnitude in Richter × 10.
    EarthquakeMagnitude,
    /// Flood gauge level in cm × 10.
    FloodGaugeCm,
    /// Regional hospital admission rate index × 100.
    HealthIndex,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TriggerDirection {
    /// Payout when oracle value > threshold.
    Above,
    /// Payout when oracle value < threshold.
    Below,
}

// ---------------------------------------------------------------------------
// Policy state machine
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyState {
    /// Coverage is active and premiums are streaming.
    Active,
    /// Trigger condition detected — awaiting confirmation window.
    TriggeredPending,
    /// Confirmation window complete — awaiting payout execution.
    ConfirmedPending,
    /// Payout has been executed to the policyholder.
    Paid { payout_ledger: u32 },
    /// Policy duration elapsed without a trigger.
    Expired,
    /// Policyholder cancelled.
    Cancelled,
    /// Premium stream ran dry — coverage lapsed.
    Lapsed,
}

// ---------------------------------------------------------------------------
// Oracle source record
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleSource {
    /// Stellar address of the oracle reporter.
    pub address: Address,
    /// Which vertical this source covers.
    pub vertical: InsuranceVertical,
    /// Reliability score 0–100. Sources below minimum are excluded from consensus.
    pub reliability_score: u32,
    /// Ledger of the most recent submission from this source.
    pub last_submission_ledger: u32,
}

// ---------------------------------------------------------------------------
// Oracle configuration (per vertical)
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    /// Minimum number of sources that must agree for a reading to be accepted.
    pub required: u32,
    /// Maximum % variance between submitted values for consensus.
    pub tolerance_pct: u32,
    /// Ledgers after which a submitted reading is considered stale.
    pub staleness_ttl: u32,
}

// ---------------------------------------------------------------------------
// Insurance verticals
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InsuranceVertical {
    CropRainfall,
    FlightDelay,
    WindDisaster,
    EarthquakeDisaster,
    FloodDisaster,
    HealthIndex,
}

// ---------------------------------------------------------------------------
// Storage key schema
// ---------------------------------------------------------------------------

#[contracttype]
pub enum DataKey {
    /// PolicyRecord keyed by PolicyId.
    Policy(PolicyId),
    /// Oracle source keyed by (vertical, source_address).
    OracleSource(InsuranceVertical, Address),
    /// Latest accepted oracle reading for (vertical, location_hash).
    AcceptedReading(InsuranceVertical, BytesN<32>),
    /// Oracle configuration per vertical.
    OracleConfig(InsuranceVertical),
    /// Contract-level configuration.
    Config,
}

// ---------------------------------------------------------------------------
// Contract configuration
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractConfig {
    pub admin: Address,
    /// Minimum oracle reliability score to participate in consensus.
    pub min_reliability_score: u32,
    /// Default confirmation window if not specified per-policy.
    pub default_confirmation_window: u32,
    /// Minimum reserve pool solvency ratio (basis points).
    pub min_solvency_ratio_bps: u32,
}

// ---------------------------------------------------------------------------
// Event topic constants
// ---------------------------------------------------------------------------

pub const EVENT_POLICY_CREATED: &str    = "PolicyCreated";
pub const EVENT_POLICY_TRIGGERED: &str  = "PolicyTriggered";
pub const EVENT_POLICY_CONFIRMED: &str  = "PolicyConfirmed";
pub const EVENT_POLICY_PAID: &str       = "PolicyPaid";
pub const EVENT_POLICY_CANCELLED: &str  = "PolicyCancelled";
pub const EVENT_POLICY_EXPIRED: &str    = "PolicyExpired";
pub const EVENT_POLICY_LAPSED: &str     = "PolicyLapsed";
pub const EVENT_ORACLE_SUBMITTED: &str  = "OracleReadingSubmitted";
pub const EVENT_ORACLE_CONSENSUS: &str  = "OracleConsensusReached";
pub const EVENT_ORACLE_FAILED: &str     = "OracleConsensusFailed";
pub const EVENT_PREMIUM_DEDUCTED: &str  = "PremiumDeducted";
