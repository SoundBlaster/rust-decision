# rust-decision
Light Decisions for Rust

The [proposed decision contract and roadmap](docs/decision-contract.md) define
the provider-independent core, SpecificationCore dogfooding and separate Jev
and OpenAI adapter boundaries. This repository is currently at the design stage.

An [offline finite-model checker](verification/README.md) verifies the proposed
Choice routing before production implementation. It runs without providers or
credentials; the Rust decision library is not implemented yet.
