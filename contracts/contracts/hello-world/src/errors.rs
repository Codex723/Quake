use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum QuakeError {
    PolicyNotFound            = 1,
    PolicyExpired             = 2,
    PolicyLapsed              = 3,
    PolicyAlreadyPaid         = 4,
    PolicyNotTriggered        = 5,
    InsufficientReserve       = 6,
    OracleNotRegistered       = 7,
    OracleReadingStale        = 8,
    OracleConsensusFailed     = 9,
    OracleAttestationInvalid  = 10,
    ConfirmationWindowActive  = 11,
    Unauthorised              = 12,
    InvalidTriggerConfig      = 13,
    ReserveUndercollateralised = 14,
    StakeLockedUp             = 15,
    AlreadyInitialized        = 16,
    InvalidConfig             = 17,
    InvalidPremiumRate        = 18,
    InvalidPayoutAmount       = 19,
    PolicyNotActive           = 20,
}
