# Contributing to Quake

Thank you for your interest in contributing to Quake. This document covers everything you need to get involved — from your first issue to submitting production-ready code. Quake has a more complex architecture than most Soroban projects due to its oracle layer, so please read the oracle-specific sections carefully before contributing to contract code.

---

## Table of contents

- [Code of conduct](#code-of-conduct)
- [Ways to contribute](#ways-to-contribute)
- [Getting started](#getting-started)
- [Project structure](#project-structure)
- [Development workflow](#development-workflow)
- [Writing Soroban contracts](#writing-soroban-contracts)
- [Working with the oracle layer](#working-with-the-oracle-layer)
- [Writing the SDK](#writing-the-sdk)
- [Testing](#testing)
- [Pull request process](#pull-request-process)
- [Issue guidelines](#issue-guidelines)
- [Security vulnerabilities](#security-vulnerabilities)
- [Community](#community)

---

## Code of conduct

Quake follows the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/) Code of Conduct. Report unacceptable behaviour via the security email in the repository's security policy.

---

## Ways to contribute

**Code**
- Implement contract features from the roadmap
- Build oracle jobs for new data sources
- Fix bugs and improve test coverage
- Optimise Soroban compute unit usage

**Research**
- Propose and model trigger threshold calibration for new verticals
- Audit the oracle consensus mechanism
- Research anchor integrations for new markets
- Model reserve pool solvency under stress scenarios

**Documentation**
- Improve guides for policyholders, LP stakers, and oracle operators
- Write integration tutorials

**Community**
- Answer questions in GitHub Discussions
- Review open pull requests
- Report bugs with clear reproduction steps

---

## Getting started

### Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| Rust | `>=1.74` | Soroban contract development |
| `soroban-cli` | latest | Contract build, deploy, invoke |
| Node.js | `>=18` | Quake.js SDK and backend |
| pnpm | `>=8` | SDK package management |
| Docker | any | Local Stellar testnet |
| Python | `>=3.10` | Oracle job scripts (optional) |

### Install Rust and Soroban toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
cargo install --locked soroban-cli
```

### Clone and build

```bash
git clone https://github.com/your-org/quake
cd quake
cargo build
```

### Start a local testnet

```bash
docker run --rm -it \
  -p 8000:8000 \
  stellar/quickstart:latest \
  --testnet \
  --enable-soroban-rpc
```

### Run the full test suite

```bash
cargo test
```

---

## Project structure

```
quake/
├── contracts/
│   └── quake/
│       ├── src/
│       │   ├── lib.rs                # Contract entry point
│       │   ├── policy.rs             # PolicyRecord type + storage
│       │   ├── oracle.rs             # OracleAggregator logic
│       │   ├── trigger.rs            # Trigger evaluation engine
│       │   ├── payout.rs             # PayoutEngine
│       │   ├── reserve.rs            # ReservePool + LP staking
│       │   ├── premium.rs            # PremiumStream module
│       │   ├── events.rs             # Event definitions
│       │   ├── errors.rs             # QuakeError enum
│       │   └── types.rs              # Shared types
│       ├── Cargo.toml
│       └── tests/
│           ├── create_policy.rs
│           ├── oracle_consensus.rs
│           ├── trigger_evaluation.rs
│           ├── payout_execution.rs
│           └── reserve_pool.rs
├── oracle/
│   ├── jobs/
│   │   ├── noaa_rainfall.ts          # Acurast job: NOAA rainfall
│   │   ├── flightaware_delay.ts      # Acurast job: flight delay
│   │   ├── noaa_hurricane.ts         # Acurast job: hurricane wind speed
│   │   └── usgs_earthquake.ts        # Acurast job: earthquake magnitude
│   ├── src/
│   │   ├── aggregator.ts             # Off-chain aggregation helper
│   │   └── validator.ts              # TEE attestation validator
│   └── README.md
├── backend/
│   ├── src/
│   │   ├── routes/
│   │   ├── services/
│   │   ├── models/
│   │   └── middleware/
│   └── package.json
├── frontend/
│   └── src/
├── sdk/
│   └── src/
├── docs/
├── CONTRIBUTING.md
├── README.md
└── LICENSE
```

---

## Development workflow

Quake uses a standard fork-and-branch workflow.

### 1. Fork and clone

```bash
git clone https://github.com/YOUR_USERNAME/quake
cd quake
git remote add upstream https://github.com/your-org/quake
```

### 2. Create a branch

```
feat/short-description        # new feature
fix/short-description         # bug fix
oracle/short-description      # oracle job or aggregator change
docs/short-description        # documentation only
test/short-description        # tests only
refactor/short-description    # no behaviour change
```

### 3. Commit conventions

Use imperative mood, reference the component in the subject:

```
# Good
[contract] Add confirmation window to trigger evaluation
[oracle] Add Acurast job for NOAA hourly rainfall
[sdk] Implement createPolicy with trigger config builder

# Bad
fixed oracle stuff
```

### 4. Sync with upstream

```bash
git fetch upstream
git rebase upstream/main
```

### 5. Open a pull request

Push and open a PR against `main`. Use `Closes #123` in the description. Fill in the PR template completely.

---

## Writing Soroban contracts

### Style conventions

- `snake_case` for all functions and variables, `PascalCase` for types
- Every public function must have a doc comment: purpose, parameters, panics, events emitted
- Use `quake_sdk::panic_with_error!` with a typed `QuakeError` — never bare `panic!`
- Keep public contract functions thin — delegate logic to internal modules

### The oracle module is special

The `oracle.rs` module is the most security-critical piece of the codebase. Rules that apply specifically here:

- Never modify consensus logic without a corresponding adversarial test
- Every change to `OracleAggregator` requires review from two maintainers regardless of size
- All oracle-related constants (`MIN_SOURCES`, `DEFAULT_TOLERANCE_PCT`, `STALENESS_TTL`) must be documented with the reasoning behind their default values
- Do not add new oracle source types without updating the source registry and reliability scoring logic

### Error handling

All errors must be variants of `QuakeError`:

```rust
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
}
```

### Events

All state-changing functions must emit typed events. Event schemas are public API — once deployed, they are immutable.

---

## Working with the oracle layer

The oracle layer is the most unique part of Quake and the area most likely to need contributor attention.

### Acurast TEE jobs

Oracle jobs live in `/oracle/jobs/`. Each job is a TypeScript script that runs inside an Acurast TEE on mobile hardware. Jobs:

1. Fetch data from an external API
2. Process and scale the reading to the correct integer format
3. Submit the reading to the `OracleAggregator` contract via a signed Stellar transaction
4. Produce a TEE attestation proving the script ran unmodified

When writing a new oracle job:

- Scale all values to integers (no floats on-chain). Rainfall in mm × 100, wind speed in km/h × 10, delay in minutes × 1.
- Include the data source URL and fetch timestamp in the submitted reading
- Handle API errors gracefully — a failed fetch should not submit a zero reading
- Test locally using the Acurast simulator before deploying

### Submitting a new oracle source

To add a new data source to an existing vertical:

1. Create a new job file in `/oracle/jobs/`
2. Register the source in the `OracleAggregator` via `register_oracle_source()`
3. Set an initial `reliability_score` of 50 (neutral)
4. Add the source to the vertical's `OracleConfig`
5. Add tests in `contracts/quake/tests/oracle_consensus.rs` covering the new source

### Reliability score calibration

Reliability scores decay and recover automatically based on oracle behaviour. When evaluating a new data source before adding it:

- Run the source against at least 30 days of historical data
- Compare readings against existing accepted sources
- Document the mean absolute error in the PR description

---

## Writing the SDK

The Quake.js SDK lives in `/sdk` and is written in TypeScript with strict mode.

### Conventions

- `camelCase` for functions and variables, `PascalCase` for classes and types
- All public methods and types must have JSDoc comments
- Explicit return types on all public API surfaces
- No `any` — use `unknown` and narrow explicitly

### Building

```bash
cd sdk
pnpm install
pnpm build
pnpm test
```

---

## Testing

### Contract tests

Live in `contracts/quake/tests/`. Use the Soroban test environment — no network required.

Every PR touching contract logic must include:

- Happy path test
- All relevant `QuakeError` variants
- Edge cases (zero amounts, boundary ledgers, oracle threshold boundaries)
- At least one adversarial oracle test (e.g. single oracle submitting false data should not trigger payout)

```bash
cargo test -p quake
```

### Oracle tests

Oracle job tests live in `/oracle/jobs/__tests__/`. Use mocked HTTP responses.

```bash
cd oracle
pnpm test
```

### Integration tests

Require Docker running with a local Stellar testnet.

```bash
cargo test --test integration
```

Integration tests are required for any change to `oracle.rs`, `trigger.rs`, or `payout.rs`.

### Adversarial oracle tests (required for oracle changes)

Any change to the `OracleAggregator` must include tests for:

- Single malicious oracle submitting an out-of-band reading (should be rejected)
- All oracles submitting at the same wrong value (should trigger — this is the expected N-of-M behaviour)
- Stale reading detection (reading older than `staleness_ttl` should be rejected)
- Attestation failure (TEE attestation mismatch should reject the reading)

---

## Pull request process

1. `cargo test` and `pnpm test` must pass locally before opening a PR
2. All PRs require one approving review from a maintainer
3. Maintainers may request changes — address with new commits, do not force-push during review
4. Maintainers squash-merge approved PRs into `main`

### PR checklist

- [ ] Code compiles without warnings
- [ ] All existing tests pass
- [ ] New tests added for new behaviour
- [ ] Adversarial oracle tests included (if touching oracle layer)
- [ ] Doc comments updated for changed public API
- [ ] PR description links the issue it resolves

---

*Quake is part of the ecosystem. Built on [Stellar](https://stellar.org) and [Soroban](https://soroban.stellar.org).*
