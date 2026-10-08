# Choice routing verification — 2026-10-08

Executed the offline checker at source commit `3c481eb` with standard-library
Python. [report.json](report.json) records exact source/contract digests, Python
version, checkout revision, invocation and reproducible mutation witnesses.

| Mode | Consistent full inputs | Representative inputs | Reachable representative states | State/action pairs |
| --- | ---: | ---: | ---: | ---: |
| Primary numeric routing | 5,160,906 | 1,665 | 90,387 | 388,908 |
| Conservative threshold extension | 5,898,168 | 1,719 | 96,165 | 413,748 |

P1–P11 passed in the finite routing abstraction. P12's finite-rank checks passed;
termination remains conditional on eventual backend event or observable
cancellation/expiry and fair core progression. This is not a real backend test.

All six faulty controls were detected: confidence bypass (P8), repeated Await
invocation (P6), authentication fallback (P7), missing-evidence acceptance (P4),
late Done rewrite (P3), and nondeterministic Request (P2). Six regression tests
passed, including JSON witness replay and dormant-plan graph equivalence cases.

The full-input count is enumerated coverage under input partitions. Reachability
counts use the [documented reduction](../../README.md); full-state traces were
not individually explored for every input. The results do not establish numeric
validator correctness, production Rust behavior, semantic AI accuracy or real
HTTP timeout/cancellation behavior. CI must generate fresh evidence for changes.
