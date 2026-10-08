# Finite model for Choice routing — proposal

Status: theoretical preparation, 2026-10-08. This document defines the model
to be checked against the [Choice routing contract](choice-routing-contract.md).
It contains no executable model, Rust implementation or verification result.
Baseline contract: main commit `278db49` (PR #2, including confidence validation).

## What the model establishes

The target is the deterministic decision core around an AI backend: structural
validation, acceptance and terminal routing. AI output is an input supplied by
the environment. The model does not prove semantic classification accuracy,
calibration, actual HTTP cancellation, floating-point correctness or library
implementation correctness.

Use a finite transition system M = (States, Init, Next). Init describes the
initial state; Next describes allowed single-step transitions. Start with
reachable-state exploration from Init, not an unrestricted Cartesian product
of fields. A separate robustness suite injects malformed internal states.

## State variables and finite domains

| Variable | Domain | Meaning |
| --- | --- | --- |
| phase | Request, Policy, Capability, Invoke, Await, Dispatch, Output, Evidence, Thresholds, Commit, Done | Current boundary |
| invocations | 0, 1 | Number of backend starts |
| event | none, prediction, refusal, transport_error, authentication_error, malformed_response, unsupported, timed_out, cancelled | Latched backend terminal event |
| request, policy, capability | NE, S, V, U, C | Validation assessments |
| identity, distribution, confidence, selection | NE, S, V, U, C | Prediction validation assessments |
| evidence, thresholds | NE, S, V, U, C | Acceptance assessments |
| output_cursor | 0, 1, 2, 3 | Next output validator in declared order |
| fallback_config | absent or any subset of three allowed triggers | Validated caller fallback routing configuration |
| confidence_input | valid_present, invalid_present, declared_absent, unsupported, contradictory | Abstract advertised confidence |
| requires_confidence | false, true | Whether caller requires confidence |
| cancelled_observed, expired_observed | false, true | Sticky observations at boundaries |
| candidate | none or a terminal outcome with reason/value tag | Proposed but unpublished result |
| published | none or a terminal outcome with reason/value tag | Committed public result |

NE means not evaluated; S/V/U/C mean satisfied/violated/unknown/conflict.
Outcome categories are Accepted, Abstained, Fallback, Failed and Cancelled.
Use distinct finite reason tags from the routing contract; do not collapse
missing evidence, conflicting evidence, rejection and provider refusal.
Each trigger subset contains provider_refusal, missing_evidence and/or
policy_rejected, giving eight subsets plus absent. An empty configured subset
is distinct configuration provenance but has the same routing behavior as absent.

Application values are represented by selected_value and fallback_value tags.
They may refer to the same application label; the outcome category still
distinguishes acceptance from fallback. Option IDs, numeric values and raw
source text are outside this routing abstraction. Validation must establish
their mapping before those tags are available. This abstraction preserves only
properties depending on routing assessments and outcome provenance; it does
not establish arbitrary-option or numerical correctness.

An evaluation log is a finite sequence of (rule ID, assessment) entries. Each
of the nine validation rules is evaluated at most once, so the log has length
at most nine. Separate reached-phase markers from rule evaluations. Backend
waiting stutters do not append unbounded trace entries in this finite model.

## Initial state and environment

Init sets phase=Request, invocations=0, event=none, every assessment=NE,
output_cursor=0, both observation flags=false, candidate=none, published=none
and the evaluation log empty. Caller configuration and abstract input facts
are chosen once for an operation and remain fixed.

Invalid caller configuration is represented by policy=V when evaluated. No
fallback may be used before policy=S. Invalid fallback values or forbidden
trigger configurations are included in invalid-policy input partitions.

Environment actions may make cancellation or expiry observable at a boundary.
Observed flags remain true until terminal commitment. They model observations,
not the physical arrival time of cancellation or wall-clock precision. A valid
pending invocation may receive one terminal backend event. That event is latched
and cannot be replaced by another event. Duplicate terminal events are adapter
contract violations tested separately, not ordinary valid environment behavior.

For safety exploration the environment may never deliver an event. For a
conditional termination check, assume it eventually delivers an event or an
observable cancellation/deadline and that core progression is weakly fair.
State the assumption in the result; it is not a guarantee of a real backend.

## Boundary override and terminal commitment

At every non-Done boundary, first observe the environment flags. If cancellation
is observed, atomically publish Cancelled and enter Done. Otherwise, if expiry
is observed, atomically publish Failed(timed_out) and enter Done. Neither path
evaluates the boundary's ordinary rule or starts inference.

With neither override, execute the phase action below. Commit atomically
publishes candidate and enters Done. Done has only a stuttering transition:
published result, log and invocation count cannot change. A cancellation first
observed after Done cannot replace an outcome. Commit is the model's linearization
point; implementation must identify its corresponding single-publication point.

## Ordinary phase transitions

All rows below require no observed override. Each assessment rule appends
exactly one entry to the evaluation log. Failure routing creates a candidate
and moves to Commit, retaining the failing assessment in its reason evidence.

| Phase | Action | Next phase / candidate |
| --- | --- | --- |
| Request | Evaluate RequestWellFormed | S → Policy; otherwise Failed(request_validation) → Commit |
| Policy | Evaluate CallerPolicyWellFormed | S → Capability; otherwise Failed(policy_validation) → Commit |
| Capability | Evaluate BackendSupportsRequest | S → Invoke; otherwise Failed(capability_validation) → Commit |
| Invoke | Require invocations=0 and prior assessments S; set invocations=1 | Await |
| Await | No event | Stutter without inference or validation |
| Await | Latch one backend event | Dispatch |
| Dispatch | prediction | Output, output_cursor=0 |
| Dispatch | refusal | Resolve provider_refusal with fallback table → Commit |
| Dispatch | cancelled | Cancelled candidate → Commit |
| Dispatch | Other terminal event | Corresponding Failed candidate → Commit |
| Output | Evaluate validator at output_cursor | S → advance cursor; last S → Evidence; otherwise Failed(output_validation) → Commit |
| Evidence | Evaluate AcceptanceEvidencePresent | S → Thresholds; otherwise route missing/conflicting evidence → Commit |
| Thresholds | Evaluate AcceptancePolicySatisfied | Use acceptance table → Commit |
| Commit | Publish candidate once | Done |

Output validator order is AnswerMatchesRequest, ProbabilityDistributionValid,
AdvertisedConfidenceValid, SelectedOptionConsistent. This proposal resolves the
implementation order explicitly; it must not weaken any validator. Stop at the
first non-S assessment; later validators remain NE. "Every prediction validates
confidence" means every prediction reaching that structural validator, before
acceptance; an earlier structural failure may stop first without acceptance.

Backend cancelled counts as an observed cancellation when the latched event is
dispatched: it has cancellation priority over expiry observed at that boundary.
The override guard includes event=cancelled in Dispatch, so the ordinary
Dispatch cancelled row cannot lose to a simultaneous deadline observation.

Resolve acceptance reasons as in the routing contract. In particular, evidence
C routes conflicting_evidence and never fallback; evidence V/U routes
missing_evidence. If evidence is not S, thresholds remains NE. For the initial
numeric evaluator, thresholds after evidence S is restricted to S or V.
A separate conservative-extension exploration includes thresholds U/C to check
the contract's future-compatible abstention routes, labeled as such.

## Consistent input partitions

Specify assumptions separately from derived invariants:

- invalid_present confidence forces confidence=V when evaluated, independent
  of requires_confidence and thresholds;
- valid_present or declared_absent confidence yields confidence=S;
  unsupported/contradictory confidence yields U/C and structural failure;
- after valid output, declared_absent plus requires_confidence gives evidence
  V or U (verified absence vs insufficient evidence), never S;
- declared_absent without a confidence requirement does not itself fail
  evidence; other policy-required evidence can still be missing;
- advertised numeric confidence is validated structurally even if no confidence
  threshold is configured;
- only policy=S configurations can enable fallback, and only the three permitted
  reasons can use it;
- all assessments begin NE and acquire a value only when their rule executes.

The finite model nondeterministically samples validator assessments subject to
these partitions. Their correctness against actual requests/predictions must
be verified separately; declaring them as assumptions does not prove the
numeric or parser implementation. Additional partitions can refine the model
without silently changing the accepted outcome contract.

## Properties and future verification artifacts

| ID | Property | Verification approach |
| --- | --- | --- |
| P1 | Every reachable non-Done state has a next transition | Reachable-state enumeration, including Await stutter |
| P2 | Fixed state and fixed environment action yield one core successor | Check guarded transitions for gaps/overlap |
| P3 | Published result is written once and immutable in Done | State invariant |
| P4 | Accepted implies request/policy/capability, all output validators, evidence and thresholds are S | State invariant |
| P5 | Accepted implies prediction event and invocations=1 | State invariant |
| P6 | invocations never exceeds one; Await never starts inference | State invariant and transition check |
| P7 | Fallback implies policy=S, explicit mapped value and matching allowed trigger | State invariant |
| P8 | Invalid advertised confidence cannot reach Accepted, even under empty policy | Partition-aware exploration |
| P9 | Observed cancellation wins over observed expiry at a boundary | Transition check, including backend cancelled |
| P10 | Logged evaluations match actual executed rules; skipped rules remain NE | State invariant |
| P11 | Required confidence absence cannot reach Accepted | Partition-aware exploration |
| P12 | Event or cancellation/expiry eventually permits Done under stated fairness assumptions | Conditional termination check |

P1 is progress availability, not termination: a permanently waiting backend
satisfies P1 via stutter. P2 conditions on environment input; different AI
predictions legitimately lead to different results. Properties about threshold
monotonicity and option permutation require paired input models or separate
numeric/mapping tests; they do not follow from arbitrary S/V assessment tags.

For each violation retain the shortest available counterexample: initial input
partition, phase/event sequence, rule assessments, candidate and published
outcome. For a successful future run retain the model version/digest, contract
revision, checker version/command, assumptions, explored-state count and property
results. No such counts or results exist in this documentation PR.

Before claiming proof, check that deliberately faulty model variants produce
counterexamples: accepting invalid confidence; invoking again while Await;
fallback on authentication failure; accepting without required evidence; and
rewriting Done on late cancellation. Keep invariant definitions independent of
the routing function so a checker does not merely compare a function to itself.

## Next implementation boundary

Review this state model and its abstraction limits. The next implementation
task is a small offline model checker or finite-state specification, separate
from production RustDecision and hosted inference. Map each model rule to its
future SpecificationCore implementation and check implementation parity later.
