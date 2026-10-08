# Choice routing verification — 2026-10-08

Executed the offline checker at source commit `06d77a3` with standard-library
Python. [report.json](report.json) records exact source/contract digests, Python
version, checkout revision, invocation and reproducible mutation witnesses.

| Mode | Consistent full inputs | Representative inputs | Reachable representative states | State/action pairs |
| --- | ---: | ---: | ---: | ---: |
| Primary numeric routing | 5,160,906 | 1,665 | 90,387 | 388,908 |
| Conservative threshold extension | 5,898,168 | 1,719 | 96,165 | 413,748 |

P1–P11 passed in the finite routing abstraction. P12's finite-rank checks passed;
termination remains conditional on eventual backend event or observable
cancellation/expiry and fair core progression. This is not a real backend test.

All seven faulty controls were detected: confidence bypass (P8), repeated Await
invocation (P6), authentication fallback (P7), missing-evidence acceptance (P4),
authentication disguised as allowed provider refusal (P7), late Done rewrite
(P3), and nondeterministic Request (P2). Seven regression tests
passed, including JSON witness replay and dormant-plan graph equivalence cases.

The full-input count is enumerated coverage under input partitions. Reachability
counts use the [documented reduction](../../README.md); full-state traces were
not individually explored for every input. The results do not establish numeric
validator correctness, production Rust behavior, semantic AI accuracy or real
HTTP timeout/cancellation behavior. CI must generate fresh evidence for changes.

## Rust implementation parity

[rust-parity.json](rust-parity.json) records the full comparison at source commit
`adb763d`, with source/lockfile/test/harness digests and the compiled binary digest.
Every offered transition at every reachable representative state matched the
shared Rust engine: primary 90,387 states / 388,908 transitions; conservative
extension 96,165 states / 413,748 transitions.

Fifteen Rust fixture/boundary tests passed on stable and Rust 1.85; the default
(non-verification) API also passed all fifteen tests. Clippy with warnings denied
passed on both toolchains. A deliberately wrong successor process was rejected
by the parity comparator with a retained failure report. The fixture example
returned `Accepted(Policy); 9 evaluated rules` without any provider request.

The named numeric Specifications are tested separately from the supplied finite
assessment plans. No live provider, wire parsing, calibration, model provenance
or interruption of blocking I/O is established by this evidence.

## Numerical permutation correction

Independent review found order-dependent sequential probability summation.
Source commit `3092f1c` validates individual bounds first, sums a private copy in
ascending numerical order with Neumaier compensation, and applies epsilon
unchanged. Caller option order and ID/value mapping remain unchanged.

The two positive permutation regressions failed before the fix. Afterward,
18 Rust tests passed on stable and Rust 1.85, including all 36 independent
request/probability order combinations for each of four distributions: valid
at zero tolerance, valid at the default tolerance boundary, and invalid beyond
each tolerance. Clippy passed on both toolchains; default-feature tests and
format/diff checks passed.

The routing engine and finite-model checker were not changed. The earlier
`rust-parity.json` remains evidence for its recorded source revision; fresh CI
produces a separate report for the corrected PR head. Numeric regressions,
not finite assessment parity, detect this numerical defect.
