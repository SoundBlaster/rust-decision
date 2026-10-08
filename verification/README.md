# Offline Choice model checker

This standard-library Python checker implements the
[finite routing model](../docs/choice-finite-model.md), not the Rust library.
It makes no network requests and uses no credentials or model providers.
Python 3.9+ is supported; CI uses Python 3.11. No virtual environment or package
installation is required because the checker has no external dependencies.

```sh
python3 -m unittest discover -s verification -p 'test_*.py'
python3 verification/choice_model.py --output verification-output/report.json
```

The command exits nonzero if either valid model has a violation or any faulty
control is not detected. It writes the report before returning. Reports contain
source/contract SHA-256 digests, checkout revision, checker/Python versions,
counts, outcome coverage and counterexamples. A checkout revision alone does
not identify uncommitted source: the recorded digests identify actual inputs.
CI retains its own report, including on a verification failure.

## Exploration and reduction

Every nine-entry assessment plan in {S,V,U,C} is enumerated. Consistency
constraints bind advertised confidence to its planned structural assessment,
and exclude acceptance of absent required confidence. The primary model also
restricts thresholds after satisfied evidence to S/V. The separately reported
conservative extension permits U/C threshold assessments.

For each plan, all compatible confidence kinds, both confidence requirements
and all nine fallback configurations are considered. Only dormant plan entries
are quotiented out:

1. If structural rule i is the first non-S rule, later rules cannot execute.
   Retain its index/outcome and all immutable non-plan dimensions.
2. If structural rules pass but evidence is non-S, thresholds cannot execute.
   Retain evidence and all immutable non-plan dimensions.
3. Otherwise retain the full observable assessment sequence.

This is a trace-preserving reduction for the unmutated routing system:
ordinary execution reaches the same first failure; overrides may stop earlier,
never expose the suffix; Commit/Done cannot evaluate a skipped rule. Fixed
environment actions therefore yield the same phase, event, evaluated assessment,
outcome and trace in every member of a class. Planned suffix values remain
explicit in the representative state and witness, but are not observations.

The report's full-input count describes all enumerated consistent inputs, while
state/action counts describe representative exploration. It does not count
millions of distinct full-state traces as individually explored. Python integer
and dataclass behavior and this reduction argument are trusted checker parts;
the reduction is reviewed reasoning, not a separately machine-proved theorem.
Regression tests compare projected transition graphs for dormant-suffix cases.

Breadth-first search explores every admitted environment action at every
reachable representative state, including all backend events, no-event waiting,
both observation flags and late observations in Done. Counterexamples are
shortest within the first failing representative, not globally across all
initial inputs. The report retains the initial plan, prefix and violating action.
Tests JSON-roundtrip and replay witnesses from all seven faulty controls.

## Property coverage and limits

P1/P2 check successor availability and uniqueness for fixed inputs/actions;
P3 checks publication and terminal immutability; P4/P5 check acceptance grounds;
P6 checks invocation count and preconditions; P7 checks fallback authority;
P8/P11 check confidence partitions; P9 checks cancellation priority; P10 checks
assessment/log correspondence and skipped work on override. Additional checks
retain immutable plans and latched events and enforce deadline/commit routing.

P12 checks a strictly increasing finite phase/cursor rank on every non-stutter
transition. Only Await and Done can stutter. Termination follows conditionally
from eventual backend event/observable cancellation or expiry and fair core
progression. No unconditional backend termination is claimed.

Faulty controls deliberately bypass confidence validation, reinvoke Await,
fallback on authentication failure, disguise authentication failure as an allowed
provider refusal, accept without evidence, rewrite Done or
introduce two Request successors. All must produce a counterexample. Controls
use representative inputs to show each detector works; they do not claim full
coverage of every possible mutant after the original reduction is invalidated.

The checker does not implement floating-point validators, option mapping,
threshold monotonicity, production SpecificationCore rules, model accuracy,
actual async cancellation, or a general malformed-internal-state/duplicate-event
robustness suite. Those remain separate implementation/parity tasks. A passing
finite model is evidence about this declared abstraction only.

## Rust implementation parity

The same Rust routing engine used by `decide` has a verification-only harness.
The `verification` feature exposes its trusted finite-state inputs; applications
should use the typed Choice API without that feature. The harness supplies fixed
assessment plans, while production calls named Specifications lazily only at
reached stages over borrowed request/policy/prediction data.

```sh
cargo test --locked --all-features
cargo build --locked --example parity --features verification
python3 verification/rust_parity.py --binary target/debug/examples/parity --output verification-output/rust-parity.json
```

Parity compares the entire next state for every offered action at every reachable
representative state in both modes, including terminal stutters and Await without
reinference. This transfers routing evidence to the shared Rust engine; it does
not prove numerical validators or the adapter's assessments. Rust boundary tests
cover those separately. Real I/O, numeric calibration, provider provenance and
async cancellation still require adapter work and evidence.
