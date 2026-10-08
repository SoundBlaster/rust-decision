# rust-decision

Provider-neutral text Choice, Predicate and Score decisions for Rust, using named rules from
[SpecificationCore](https://github.com/SoundBlaster/specification-core-rs).

The first implementation provides typed local option mapping, nine deterministic
rules, acceptance thresholds, explicit abstention/fallback/failure/cancellation,
and safe rule/lifecycle traces. Rules live in separate files under `src/rules`.
No HTTP, implicit retries, provider escalation or async runtime is included.

`decide` accepts a synchronous backend and a boundary-observation callback.
Cancellation and expiry are checked before each transition and before publication;
they cannot interrupt a blocking backend call. Adapters own wire parsing,
credentials, model provenance and real transport cancellation.

```sh
cargo run --locked --example fixture
# Accepted(Policy); 9 evaluated rules — a deterministic illustration, not AI evidence
```

The [fixture example](examples/fixture.rs) maps provider IDs to a local Rust enum.
Applications can implement the same backend trait in an adapter. Acceptance
establishes contract/policy compliance, not semantic truth.

## Predicate and Score

`decide_predicate` uses `PredicateRequest`, `PredicatePolicy` and `ScalarBackend`
to return `Report<bool>`. `reject_at` and `accept_at` must be ordered finite unit
probabilities: inclusive endpoints accept false/true and the interval between
them abstains. The default pair is 0.1 / 0.9; select thresholds on labeled data.
The native yes probability is not a separate confidence estimate.

`decide_score` uses `ScoreRequest` (ordered text rubric), `ScorePolicy` and the
same scalar backend to return `Report<f64>`. Numeric levels are rubric positions
0..n-1. Validate exact unique probability coverage, normalization, finite native
confidence and agreement between score and the probability-weighted level mean.
Policy bounds and fallback values must fit the rubric; confidence requirements
preserve unavailable evidence as abstention. No rounding or normalization repair.

These methods reuse the finite lifecycle and terminal categories. Each of their
nine Specifications lives under `src/scalar/rules`. Existing Choice APIs and
backend implementations stay compatible. The scalar concrete rules have unit
regressions; Choice finite-model evidence covers the shared routing engine, not
an exhaustive proof of scalar numerical inputs.

See [scalar contracts](docs/scalar-contract.md), the [core contract](docs/decision-contract.md),
[routing contract](docs/choice-routing-contract.md), and
[verification instructions](verification/README.md).
