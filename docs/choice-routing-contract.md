# Choice routing contract v1 — proposal

Status: theoretical preparation, 2026-10-07. This document specifies a finite
routing model for [the proposed decision core](decision-contract.md). No Rust
types, network calls, Specifications or exhaustive tests are implemented here.
Formal verification of the implementation remains future work.

## Scope and vocabulary

The core evaluates an AI Choice prediction against caller acceptance policy.
It validates structural correctness and handles uncertainty; semantic accuracy
and calibration require labeled-data evaluation. SpecificationMetrics' code
eligibility rules are consumer-owned and are not part of this generic table.

A completed operation has exactly one terminal outcome:

- Accepted: a validated provider-selected application value passes policy.
- Abstained: refusal, missing acceptance evidence or policy rejection, with a
  distinct reason and no accepted application value.
- Fallback: the caller's configured value, explicitly marked as fallback and
  retaining the refusal, missing-evidence or rejection reason.
- Failed: an invalid request/policy, backend contract failure, invalid output,
  deadline expiry or operational error, with a categorized safe reason.
- Cancelled: cancellation observed at a decision boundary.

These are semantic cases, not final Rust enum signatures. No outcome implies
that a business action is authorized or that the selected label is true.

## Finite evidence states

Each validation rule produces an assessment with evidence:

| State | Meaning |
| --- | --- |
| satisfied | Required facts establish the rule |
| violated | Facts establish a violation |
| unknown | Necessary facts are missing or unsupported |
| conflict | Necessary facts support incompatible conclusions |

Not evaluated is a lifecycle state, not an unknown assessment. A trace must not
report a skipped rule as if it examined missing evidence. An unknown fact is
relevant only to rules requiring it: missing confidence has no effect on a
probability-only policy. Missing requested confidence prevents acceptance.

Request, capability and output validation require satisfied. Unknown or
conflict at these structural boundaries is a Failed contract check with its
reason retained. Missing/conflicting evidence at the acceptance boundary yields
Abstained or an explicitly configured Fallback, as specified below.

SpecificationCore's Boolean Specification interface can express the routing
guards over this typed assessment context. A Boolean false must not erase the
underlying violated/unknown/conflict reason. Establish the assessment outside
the guard, preserve it in the context and retain evidence in the result. The
exact composition and tracing facilities available in the Rust Core must be
verified before implementation; this proposal assumes no unverified Core API.

## Ordered stages and validation Specifications

1. Observe cancellation/deadline; validate the request and caller policy.
2. Validate backend capability, then invoke the backend once.
3. Observe cancellation/deadline; classify the backend terminal event.
4. For a prediction, validate question identity, options and distribution.
5. Evaluate required acceptance evidence and caller thresholds.
6. Observe cancellation/deadline before committing one terminal outcome.

There are no implicit retries or provider escalation. The backend event is one
of Prediction, Refusal, TransportFailure, AuthenticationFailure,
MalformedResponse, UnsupportedCapability, TimedOut or Cancelled. Adapter-specific
failures map to a documented safe category. Unsupported capability should be
rejected before inference when known; discovery during inference remains a
failure. A missing/duplicate answer is malformed output, not low confidence.

Named Specifications and their required inputs:

| Specification | Required inputs | Consequence of non-satisfied assessment |
| --- | --- | --- |
| RequestWellFormed | Question ID, text bounds, unique option IDs/descriptions | Failed(request_validation) |
| CallerPolicyWellFormed | Finite thresholds in [0,1], tolerance, fallback target and triggers | Failed(policy_validation) |
| BackendSupportsRequest | Explicit text/Choice capabilities and adapter limits | Failed(capability_validation) |
| AnswerMatchesRequest | One matching question/type; exact option identities | Failed(output_validation) |
| ProbabilityDistributionValid | All probabilities finite/in [0,1]; exact coverage; unit sum | Failed(output_validation) |
| SelectedOptionConsistent | Known selection and maximum/tie consistency | Failed(output_validation) |
| AcceptanceEvidencePresent | Only probability/confidence required by the caller policy | Acceptance routing table |
| AcceptancePolicySatisfied | Validated required values and thresholds | Acceptance routing table |

Missing confidence is permitted in a structurally valid prediction when the
adapter declares it unavailable; it becomes unknown only for acceptance policies
requiring confidence. An advertised confidence that is nonfinite or outside
[0,1] is invalid output. Valid numerical confidence is not evidence of
cross-provider calibration.

## Numerical and mapping rules proposed for v1

- Preserve opaque string IDs and the caller's bijection to application values.
  Labels with matching display text still require distinct IDs.
- Probability normalization tolerance is absolute: default epsilon = 1e-6,
  configurable with a finite value in [0, 1e-3]. Record the effective tolerance.
  Validate individual range bounds strictly; do not clip or renormalize values.
- A selected probability p is consistent when max_probability - p <= epsilon.
  Preserve the provider's selected ID within a tie; never replace it silently.
- A configured threshold t passes when its validated value >= t. Normalization
  tolerance does not lower the caller's acceptance threshold.
- With no acceptance thresholds, a structurally valid prediction passes the
  empty conjunction. Record that policy explicitly; it provides no statistical
  guarantee. Applications requiring review must configure their policy.
- Fallback must map to a value registered in the request's option mapping.
  A future out-of-domain fallback needs a separate output contract. Configured
  triggers are a subset of {provider_refusal, missing_evidence, policy_rejected}.
  There is no operational-error fallback in v1.

The proposed epsilon and its upper bound are design choices for review, not
vendor guarantees. Validate them against captured responses before release.

## Complete lifecycle routing table

Rows are evaluated in order at the applicable boundary. A terminal row stops
processing; downstream rules remain not evaluated.

| Priority | Boundary condition | Outcome or transition |
| --- | --- | --- |
| 1 | Cancellation observed before terminal commitment | Cancelled |
| 2 | Deadline expired at a boundary, without observed cancellation | Failed(timed_out) |
| 3 | Request or caller policy assessment is not satisfied | Failed with its validation category |
| 4 | Capability assessment is not satisfied | Failed(capability_validation) |
| 5 | Valid request/policy/capability; no backend event yet | Invoke backend, await a terminal event |
| 6 | Backend cancellation | Cancelled |
| 7 | Backend operational, timeout, capability or malformed-response failure | Failed with the mapped category |
| 8 | Backend refusal | Route reason provider_refusal through fallback table |
| 9 | Prediction; any output validation is not satisfied | Failed(output_validation) |
| 10 | Prediction; output valid | Evaluate acceptance and use acceptance table |

The awaiting state is nonterminal. Contract completeness applies to operations
with a terminal backend event or a boundary cancellation/deadline event, not to
an unresponsive backend. Event mutual exclusivity and cooperative cancellation
are backend obligations. If an adapter cannot meet them, it cannot advertise
the corresponding capability. The implementation must define its terminal
commit point; this contract does not promise interruption of completed work.
Observed cancellation wins over a simultaneously observed expired deadline.
Cancellation after commitment does not rewrite a completed outcome.

## Complete acceptance table

Output validation is already satisfied. Let E be the assessment that all
policy-required evidence is present and consistent. T is threshold assessment.

| E | T | Transition |
| --- | --- | --- |
| satisfied | satisfied | Accepted(provider-selected mapped value) |
| satisfied | violated | Route reason policy_rejected through fallback table |
| satisfied | unknown | Route reason missing_evidence through fallback table |
| satisfied | conflict | Route reason conflicting_evidence through fallback table |
| violated | not evaluated | Route reason missing_evidence through fallback table |
| unknown | not evaluated | Route reason missing_evidence through fallback table |
| conflict | not evaluated | Route reason conflicting_evidence through fallback table |

When E is not satisfied, T must remain not evaluated. Other E/T combinations
are unreachable states and must be diagnosed as an internal contract failure,
not accepted or counted as ordinary abstention. For the initial numerical
threshold evaluator, T unknown/conflict should be unreachable after E satisfied;
they have conservative routes for future supported evidence extensions.

## Complete fallback table

| Reason | Valid fallback configured with this trigger | Otherwise |
| --- | --- | --- |
| provider_refusal | Fallback(value, provider_refusal) | Abstained(provider_refusal) |
| missing_evidence | Fallback(value, missing_evidence) | Abstained(missing_evidence) |
| policy_rejected | Fallback(value, policy_rejected) | Abstained(policy_rejected) |
| conflicting_evidence | Not permitted in v1 | Abstained(conflicting_evidence) |

Trying to configure the conflicting_evidence trigger is an invalid caller
policy. Invalid requests, invalid predictions and operational failures never
enter this table. The refusal reason remains identifiable even if the caller
elects fallback. A model returning the application's label needs_review is a
normal prediction; it is not engine abstention or provider refusal.

## Formal obligations and theoretical cases

The future exhaustive check enumerates reachable combinations of finite
assessment states, backend events, cancellation/deadline observations and
fallback configurations. Numeric validators need separate property and boundary
tests; enumerating the routing states does not prove their floating-point code.

Verify these obligations:

1. Every reachable terminal state maps to exactly one outcome.
2. Accepted implies all structural checks and policy-required assessments are
   satisfied and the value comes from the selected-ID mapping.
3. Refusal, malformed output, missing evidence, cancellation and deadline expiry
   cannot produce Accepted.
4. Fallback implies an explicit valid value and an allowed matching trigger.
5. Adding a required threshold cannot turn a prior rejection into acceptance
   for an unchanged prediction. Raising that threshold cannot do so either.
6. Permuting option storage with unchanged IDs/probabilities/selection leaves
   the application outcome unchanged; the externally supplied order is retained.
7. Skipped rules have no invented evaluation evidence; evaluation and lifecycle
   trace events remain distinguishable.

| Case | Expected outcome |
| --- | --- |
| Valid distribution/identity; selected p=0.8; required p>=0.7 | Accepted |
| Same prediction; required confidence absent | Abstained(missing_evidence), or configured matching fallback |
| Same prediction; required p>=0.9 | Abstained(policy_rejected), or configured matching fallback |
| Unknown selected ID, duplicate answer or nonfinite probability | Failed(output_validation) |
| Valid provider refusal | Abstained(provider_refusal), or configured matching fallback |
| Authentication failure with fallback configured | Failed(authentication), no fallback |
| Observed cancellation and expiry before commitment | Cancelled |
| p values sum to 0.9 | Failed(output_validation) |
| Two tied maximum options; provider selected one of them | Preserve selection; evaluate its acceptance evidence |
| Valid prediction selects application label needs_review and passes policy | Accepted(needs_review); consumer chooses what that label means |

These are theoretical expected cases, not executed test results. Manual review
of the table precedes implementation. Verify the realized Specifications and
their composition against the table, then evaluate AI classification accuracy
separately using independently reviewed cases.
