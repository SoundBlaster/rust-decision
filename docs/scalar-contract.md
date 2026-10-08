# Predicate and Score contract

Provider-neutral text subset, synchronous single-question execution.

## Predicate

- Request: named question, text context/instructions, optional true/false text criteria.
- Output: named native probability of true. No invented provider confidence.
- Validate finite p in [0,1]. Caller policy requires 0 <= reject_at < accept_at <= 1.
- p <= reject_at -> Accepted(false); p >= accept_at -> Accepted(true).
- Interior probability -> Abstained(PolicyRejected), or explicitly configured semantic fallback.
- Finite probability validation always precedes policy thresholds.

## Score

- Request: named question and nonempty ordered text rubric; positions are numeric levels starting at zero.
- Output: named score, exact index-keyed probabilities and provider confidence.
- Require exactly one probability per level, each finite in [0,1], sum approximately one.
- Require score finite within rubric bounds and equal to the weighted mean within epsilon.
- Epsilon is a caller-defined absolute tolerance for both checks, finite in [0,0.001], default 1e-6.
- Deterministic compensated sums preserve response order independence. No repair, clipping or renormalization.
- Native confidence, when present, must be finite in [0,1] regardless of policy.
- Explicitly unavailable confidence permits acceptance only without a confidence requirement;
  a required unavailable confidence abstains with MissingEvidence. Unsupported/conflicting
  confidence violates output validation, following the existing Choice behavior.
- Optional inclusive min/max score bounds and min confidence can reject valid output.
- Fallback numeric values must belong to the rubric's range.

## Shared execution

`ScalarBackend` is additive; existing Choice `Backend` implementations are unaffected.
`decide_scalar` validates request/policy kind agreement. Typed wrappers return bool/f64.
Capabilities are pure, local declarations of supported kinds and UTF-8/rubric limits.
Reuse the existing nine-slot lifecycle, invocation bound and pre/post-invoke observations.
Cancellation wins over simultaneous expiry, including a backend Cancelled event.
Operational errors fail; they never activate fallback. Fallback triggers exclude conflicts
and duplicates. Report traces contain rule IDs and assessments, never prompts or keys.

The new numerical Specifications are checked with concrete regressions. Existing finite
routing/parity evidence verifies the engine over supplied assessments; it does not prove
semantic classification correctness or exhaust all floating-point inputs.

## Adapter boundaries

TypeSafe maps Predicate to native Noul. OpenAI's Predicate is a separate wire contract;
this addition does not implement an OpenAI transport or claim equivalent confidence.
Adapters own native JSON state, legends, duplicates, credentials and metadata.
TypeSafe Score's native legend must match the requested index-to-description rubric.
Native TypeSafe Score confidence is required by its schema; absent confidence is malformed.
Batch and asynchronous execution remain separate future APIs.
