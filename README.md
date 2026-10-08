# rust-decision

Provider-neutral text Choice decisions for Rust, using named rules from
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

See the [core contract](docs/decision-contract.md),
[routing contract](docs/choice-routing-contract.md), and
[verification instructions](verification/README.md).
