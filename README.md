# Quake

**Parametric microinsurance protocol on Stellar.**

Quake enables non-custodial, oracle-triggered insurance payouts with no adjusters, no paperwork, and no waiting. Policyholders stream micro-premiums per second. When a real-world parameter breaches a defined threshold — rainfall too low, wind speed too high, flight delayed too long — the Soroban contract verifies N-of-M oracle consensus and executes an automatic payout directly to the policyholder's wallet. Built on Soroban smart contracts with native Stellar path payments and anchor integration.

---

## Table of contents

- [Why Quake](#why-quake)
- [How it works](#how-it-works)
- [Supported verticals](#supported-verticals)
- [Architecture](#architecture)
- [Oracle layer](#oracle-layer)
- [Contract interface](#contract-interface)
- [Premium streaming](#premium-streaming)
- [Payout mechanics](#payout-mechanics)
- [SDK — Quake.js](#sdk--quakejs)
- [Fee structure](#fee-structure)
- [Security model](#security-model)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Why Quake

Traditional insurance fails the people who need it most. A smallholder farmer in Nigeria, a fisherman in Bangladesh, a market vendor in El Salvador — all of them face catastrophic climate and event risk, and none of them have access to affordable, trustworthy insurance. The barriers are structural: claims adjusters are expensive, fraud is rampant, payouts take months, and premiums require bank accounts most people don't have.

Parametric insurance removes the adjuster entirely. Instead of paying when you can prove loss, it pays when a measurable parameter crosses a threshold. The question shifts from "did you suffer?" to "did the event happen?" That question can be answered by data.

Quake puts that model on Stellar. Premiums stream in real time via Soroban smart contracts. Oracle data feeds are aggregated with N-of-M consensus to prevent manipulation. When a trigger fires, the payout executes atomically — no human in the loop, no claims process, no waiting. Stellar's anchor ecosystem means policyholders in emerging markets pay premiums and receive payouts in their local currency.

---

## How it works

### 1. Policy creation

The policyholder signs a transaction calling `create_policy()` on the Quake contract. This stores a `PolicyRecord` on Soroban encoding the coverage terms: trigger parameter, threshold value, payout amount, premium rate, and policy duration. No underwriter approval required.

### 2. Premium streaming

The policyholder funds a continuous stream into the `ReservePool` contract. Premiums deduct per-second. If streaming stops, coverage lapses automatically. Stellar path payments handle currency conversion — a farmer pays in their anchor's local asset, the pool holds USDC.

### 3. Oracle monitoring

The `OracleAggregator` contract collects readings from multiple independent oracle sources. Readings are accepted only when N-of-M sources agree within a configurable tolerance band. Each oracle source is weighted and tracked for reliability over time.

### 4. Trigger evaluation

On each oracle update, the `PolicyContract` evaluates all active policies against the latest accepted reading. If a policy's trigger condition is met, the contract marks the policy as `TriggeredPending` and starts the confirmation window.

### 5. Confirmation window

To prevent false triggers from transient data spikes, the trigger condition must persist for a configurable number of oracle updates. If the condition clears before the window closes, the policy returns to `Active`.

### 6. Automatic payout

Once confirmed, the `PayoutEngine` executes atomically: marks the policy as `Paid`, transfers the payout amount from the reserve pool to the policyholder's Stellar address, and emits an on-chain event. No claims form. No waiting.

---

## Supported verticals

### Crop insurance
- **Parameter:** Monthly or weekly rainfall (mm)
- **Trigger:** Rainfall below drought threshold or above flood threshold
- **Oracle sources:** NOAA, Acurast weather feeds, OpenWeatherMap via TEE
- **Target market:** Smallholder farmers in Sub-Saharan Africa, South Asia
- **Typical premium:** $1–5/month streamed per second
- **Typical payout:** $200–$1,000 per season

### Flight delay
- **Parameter:** Actual departure/arrival delay (minutes)
- **Trigger:** Delay exceeds configured threshold (e.g. 180 minutes)
- **Oracle sources:** FlightAware, OAG Aviation, Cirium via Acurast TEE
- **Target market:** Global travelers, frequent fliers
- **Typical premium:** $3–10 per flight
- **Typical payout:** $100–$500 per delayed flight

### Disaster cover
- **Parameter:** Wind speed (km/h), earthquake magnitude (Richter), flood gauge level (m)
- **Trigger:** Parameter exceeds severity threshold
- **Oracle sources:** NOAA National Hurricane Center, USGS Earthquake Hazards, Copernicus Emergency Management
- **Target market:** Coastal communities, Caribbean and Pacific islands
- **Typical premium:** $5–20/month
- **Typical payout:** $500–$5,000 per event

### Health index (beta)
- **Parameter:** Regional hospital admission rate index
- **Trigger:** Index exceeds epidemic threshold
- **Oracle sources:** WHO regional data, national health ministry feeds via Acurast
- **Target market:** Informal workers in high-disease-burden regions
- **Typical premium:** $2–8/month

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        Quake Protocol                            │
│                                                                  │
│  ┌─────────────────┐    ┌──────────────────┐                    │
│  │  PolicyContract │    │  OracleAggregator│                    │
│  │                 │    │                  │                    │
│  │  PolicyRecord   │◄───│  N-of-M consensus│                    │
│  │  State machine  │    │  Source registry │                    │
│  │  Trigger logic  │    │  Staleness check │                    │
│  └────────┬────────┘    └──────────────────┘                    │
│           │                      ▲                              │
│  ┌────────▼────────┐             │ oracle reports               │
│  │  PayoutEngine   │    ┌────────┴─────────┐                   │
│  │                 │    │  Acurast TEE Jobs │                   │
│  │  Atomic payout  │    │  Band Protocol    │                   │
│  │  Path payment   │    │  Direct API feeds │                   │
│  └────────┬────────┘    └──────────────────┘                   │
│           │                                                     │
│  ┌────────▼────────┐                                            │
│  │  ReservePool    │                                            │
│  │                 │                                            │
│  │  Premium intake │                                            │
│  │  Payout reserve │                                            │
│  │  LP staking     │                                            │
│  └─────────────────┘                                            │
└──────────────────────────────────────────────────────────────────┘
         ▲                                      ▼
   Policyholder wallet               Policyholder wallet
   (premium stream in)               (payout out)
```

### Core components

**`PolicyContract`** — The primary Soroban contract. Stores all `PolicyRecord` entries. Evaluates trigger conditions against oracle data. Manages the policy state machine. Emits events for all state transitions.

**`OracleAggregator`** — Receives and validates readings from multiple oracle sources. Implements N-of-M consensus. Tracks source reliability scores. Rejects stale or out-of-band readings.

**`PayoutEngine`** — Internal module handling payout execution. Constructs Stellar path payment for multi-asset conversion. Executes atomically within a single ledger. Cannot be called externally.

**`ReservePool`** — Holds premiums collected from policyholders and staked capital from liquidity providers. Maintains a solvency ratio check before each payout. LPs earn yield from premium income when no claims occur.

---

## Oracle layer

The oracle layer is the most critical component of Quake. A single oracle is a manipulation vector. N-of-M consensus with weighted sources is the solution.

### Oracle sources by vertical

**Crop insurance:** NOAA Global Surface Summary, OpenWeatherMap, Meteomatics, Band Protocol BandChain weather feeds — all via Acurast TEE.

**Flight delay:** FlightAware AeroAPI, OAG FlightStatus, Cirium, OpenSky Network — all via Acurast TEE.

**Disaster cover:** NOAA National Hurricane Center, USGS Earthquake Hazards, Copernicus Emergency Management, Pacific Disaster Center, Band Protocol disaster index.

### N-of-M consensus

Each `OracleConfig` defines:

```
sources:        [OracleSource; N]
required:       u32                 // M — minimum agreeing sources
tolerance_pct:  u32                 // max % variance between readings
staleness_ttl:  u32                 // ledgers before reading is considered stale
```

A reading is accepted only when M or more sources have submitted within `staleness_ttl` ledgers and all submitted values are within `tolerance_pct` of each other.

### Acurast TEE integration

Acurast runs oracle scripts inside Trusted Execution Environments (TEEs) on mobile hardware. The TEE produces a cryptographic attestation proving the script ran unmodified against real data. Quake verifies this attestation on-chain before accepting a reading.

### Oracle reliability scoring

Each oracle source has a `reliability_score` (0–100) that decays when the source submits late or out-of-band, and recovers when it submits accurate, timely data. Sources below a minimum score are automatically excluded from consensus.

---

## Contract interface

### `create_policy`

```rust
pub fn create_policy(
    env: Env,
    policyholder: Address,
    vertical: InsuranceVertical,
    coverage_asset: Address,
    payout_amount: i128,
    premium_asset: Address,
    premium_rate: i128,             // per ledger
    duration_ledgers: u32,
    trigger_config: TriggerConfig,
    location: Option<String>,
) -> PolicyId;
```

---

### `submit_oracle_reading`

```rust
pub fn submit_oracle_reading(
    env: Env,
    oracle: Address,
    vertical: InsuranceVertical,
    location_hash: BytesN<32>,
    value: i128,
    timestamp: u64,
    attestation: Bytes,
) -> OracleReadingId;
```

---

### `evaluate_triggers`

```rust
pub fn evaluate_triggers(
    env: Env,
    vertical: InsuranceVertical,
    location_hash: BytesN<32>,
) -> Vec<PolicyId>;
```

---

### `confirm_payout`

```rust
pub fn confirm_payout(
    env: Env,
    policy_id: PolicyId,
) -> PayoutResult;
```

---

### `cancel_policy`

```rust
pub fn cancel_policy(
    env: Env,
    policy_id: PolicyId,
    policyholder: Address,
) -> ();
```

---

### `stake_reserve`

```rust
pub fn stake_reserve(
    env: Env,
    provider: Address,
    asset: Address,
    amount: i128,
) -> StakeId;
```

---

### Key types

```rust
pub struct PolicyRecord {
    pub id: PolicyId,
    pub policyholder: Address,
    pub vertical: InsuranceVertical,
    pub coverage_asset: Address,
    pub payout_amount: i128,
    pub premium_rate: i128,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub trigger_config: TriggerConfig,
    pub location_hash: BytesN<32>,
    pub state: PolicyState,
}

pub struct TriggerConfig {
    pub parameter: OracleParameter,
    pub threshold: i128,
    pub direction: TriggerDirection,   // Above or Below
    pub confirmation_window: u32,
}

pub enum PolicyState {
    Active,
    TriggeredPending { confirmed_at: u32 },
    Paid { payout_ledger: u32 },
    Expired,
    Cancelled,
    Lapsed,
}

pub enum InsuranceVertical {
    CropRainfall,
    FlightDelay,
    WindDisaster,
    EarthquakeDisaster,
    FloodDisaster,
    HealthIndex,
}
```

---

## Premium streaming

Premiums deduct from the policyholder's wallet per-second and accumulate in the `ReservePool`. Stellar path payments handle currency conversion atomically — a farmer in Kenya streams KES (via an M-PESA anchor) and the pool receives USDC in the same transaction. If the policyholder's balance is insufficient for 3 consecutive ledgers, the policy lapses automatically.

---

## Payout mechanics

Before any payout, the `PayoutEngine` checks that the `ReservePool` has sufficient capital. A confirmed payout executes in a single Soroban transaction: marks the policy as `Paid`, deducts the payout amount from the pool, executes a Stellar path payment to the policyholder in their preferred asset, and emits a `PolicyPaid` event.

---

## SDK — Quake.js

### Installation

```bash
npm install @quake-protocol/sdk
```

### Create a policy

```typescript
import { QuakeClient } from '@quake-protocol/sdk';

const quake = new QuakeClient({ network: 'mainnet' });

const policyId = await quake.createPolicy({
  wallet,
  vertical: 'crop_rainfall',
  payoutAmount: 500_00,
  durationDays: 90,
  trigger: {
    parameter: 'rainfall_mm',
    threshold: 50,
    direction: 'below',
    confirmationWindow: 2,
  },
  location: { lat: 9.0579, lon: 7.4951 },
});
```

### Listen for payouts

```typescript
quake.on('policy:paid', (event) => {
  console.log(`Payout executed: ${event.policyId} — ${event.amount} USDC`);
});
```

---

## Fee structure

| Cost | Who pays | Amount |
|---|---|---|
| Policy creation tx | Policyholder | ~0.001 XLM once |
| Premium per ledger | Policyholder | Configured rate |
| Oracle submission | Oracle operators | ~0.001 XLM per reading |
| Trigger evaluation | Permissionless caller | ~0.002 XLM |
| Payout execution | ReservePool | ~0.001 XLM |

No protocol fee in v1. A governance-controlled fee on payouts may be introduced in v2.

---

## Security model

### What Quake can do

- Deduct premiums up to the configured `premium_rate` per ledger
- Execute a payout up to `payout_amount` when trigger conditions are confirmed
- Lapse a policy when the premium stream runs dry

### What Quake cannot do

- Pay out more than `payout_amount` per policy per trigger
- Accept a trigger from a single oracle source (N-of-M required)
- Execute a payout without the confirmation window elapsing
- Be paused or modified by any admin key (v1)

### Threat model

**Oracle manipulation** — N-of-M consensus means a single compromised oracle cannot trigger a false payout. Acurast TEE attestations make manipulation cryptographically detectable.

**Flash oracle attacks** — The confirmation window prevents a transient data spike from triggering a payout. The condition must persist across multiple oracle update rounds.

**Reserve pool drain** — Solvency checks run before every payout. LP stakes provide a capital buffer. Payouts are queued, not dropped, if the pool is temporarily undercollateralised.

### Audit status

- [ ] Internal review — in progress
- [ ] External audit — planned pre-mainnet
- [ ] Bug bounty — planned post-audit

---

## Roadmap

### v0.1 — Testnet alpha
- [ ] Core `PolicyContract` with crop rainfall vertical
- [ ] `OracleAggregator` with N-of-M consensus
- [ ] Acurast TEE oracle job for NOAA weather data
- [ ] `ReservePool` with premium streaming
- [ ] Testnet deployment

### v0.2 — Multi-vertical
- [ ] Flight delay vertical
- [ ] Wind disaster vertical
- [ ] Quake.js SDK
- [ ] Backend API and webhook relay

### v0.3 — LP staking + frontend
- [ ] Liquidity provider staking and yield distribution
- [ ] Policyholder dashboard
- [ ] LP dashboard

### v1.0 — Mainnet
- [ ] External security audit
- [ ] Bug bounty program
- [ ] Mainnet deployment
- [ ] Anchor integrations (M-PESA, bKash, Wave)

### v2.0 — Dynamic pricing + governance
- [ ] Decentralised actuarial pricing oracle
- [ ] Protocol fee governance
- [ ] Community oracle node program
- [ ] Health index vertical

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for full guidelines.

```bash
git clone https://github.com/your-org/quake
cd quake
cargo build
cargo test
```

---

## License

MIT — see [LICENSE](./LICENSE).

---

*Built on [Stellar](https://stellar.org) and [Soroban](https://soroban.stellar.org).*
