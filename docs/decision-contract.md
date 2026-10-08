# RustDecision contract proposal

Status: theoretical preparation, 2026-10-07. No Rust API or backend is implemented
by this proposal. Package name: `rust-decision`; Rust import: `rust_decision`.

## Purpose and ownership

Provide typed finite decisions, validated predictions, explicit acceptance,
abstention and caller-defined fallback. Use the author's
[specification-core-rs](https://github.com/SoundBlaster/specification-core-rs)
for named validation and acceptance rules. Keep HTTP, credentials and provider
wire formats in adapters. SpecificationMetrics owns code categories, intent
profiles, reviewed evidence and S/U accounting.

Dependency direction: RustJev depends on RustDecision; RustDecision depends on
SpecificationCore. RustDecision never depends on RustJev. A future OpenAI
adapter depends on the same core contract and does not route through RustJev.
No new OpenAI repository is required at the contract stage.

## Initial semantic surface

Implement text-only Choice first. A request contains bounded text context,
instructions, an explicit question identity and ordered options. Each option
has a unique opaque string ID and criterion description. Application values
(including enums) stay local behind the ID mapping; the provider never
constructs an application enum. Preserve the supplied ordering.

The contract reserves future Predicate and Score capabilities without claiming
support. Noul is Jev terminology; a generic Boolean question is Predicate.
Multimodal input, native multi-question requests and scheduling are later
extensions. Unsupported capabilities must be rejected before network inference.

## Backend boundary

The backend accepts provider-independent input and returns either a prediction,
an explicit refusal, or a categorized operational failure. Exact Rust trait
signatures, async runtime and object-safety choices remain design decisions for
the implementation PR. Cancellation/deadline semantics must be specified there
before offering timeout guarantees; an async wrapper around blocking I/O alone
does not guarantee cancellation of the underlying work.

For Choice, a prediction carries:

- the provider-selected option ID;
- probabilities keyed by option ID, reconciled to request order;
- optional provider confidence with provider/model identity and semantics;
- requested and returned model IDs, request identifier and usage when available;
- adapter identity/version and native API kind.

Do not identify answers by array position when question names are available.
Reject duplicate, missing, extra or mismatched IDs and wrong answer kinds.
Validate finite probabilities in [0, 1], exact option coverage and a unit sum
within a documented numerical tolerance. Do not silently renormalize malformed
data. A selected label must be one of the maximal-probability options under a
documented tie/tolerance rule; do not replace the vendor selection silently.

Confidence is distinct from selected-option probability. A missing confidence
is missing evidence: a policy requiring it cannot accept the prediction.
Equal numeric confidence from different providers does not imply equal quality.

## Outcomes and failure semantics

Successful policy evaluation yields Accepted(value), Abstained(reason), or
Fallback(value, reason). Fallback is explicitly provided by the caller and
remains visibly a fallback, never a model-accepted answer.

Provider refusal, transport failure, authentication failure, malformed output,
timeout and cancellation remain distinguishable. Low-confidence abstention is
a different event from provider refusal. Operational failures must not be
converted to ordinary abstention without an explicit caller policy. Cancellation
terminates work without invoking a fallback. Safe errors omit credentials and
raw input/output; detailed sensitive diagnostics require a separate opt-in.

No implicit provider escalation, retries or hosted inference. The application
owns budgets, provider choice, calibration and review authority.

## SpecificationCore dogfooding

Named Specifications evaluate borrowed request or prediction contexts:

| Rule | Evidence checked |
| --- | --- |
| RequestWellFormed | Unique IDs, nonempty valid options, supported input |
| BackendSupportsRequest | Declared capability and adapter limits |
| AnswerMatchesRequest | Question identity/type and exact option coverage |
| ProbabilityDistributionValid | Finite values, range and normalization |
| SelectedOptionConsistent | Known label, maximum-probability/tie rule |
| AcceptanceEvidencePresent | Required probability/confidence is available |
| AcceptancePolicySatisfied | Caller thresholds on validated evidence |

Keep deterministic checks in SpecificationCore; backend inference remains an
explicit effect between validation stages. A passing validator establishes
contract compliance, not semantic truth or model calibration. The classifier's
ExistingSpecificationDecision and related domain rules belong in its consumer.

Trace must distinguish reached lifecycle stages from evaluated Specifications.
Record stable rule IDs, outcomes and invocation-local ordering. Trace excludes
prompts, predictions and secrets by default; transport failures do not invent
Specification evaluations that never occurred. Whether Core already supplies
the necessary trace API must be checked before designing a Core extension.

## OpenAI compatibility evidence

Checked 2026-10-07 against the official
[Decisions guide](https://developers.openai.com/api/docs/guides/decisions) and
[create reference](https://developers.openai.com/api/reference/resources/decisions/methods/create).
OpenAI has a distinct `/v1/decisions` contract: input, an array of named questions
and an array of answers. Its primitives are predicate, choice and score, with an
explicit refusal answer. Choice uses typed values and per-option probability
entries. The initial string-ID core is a deliberate subset; adapters must not
stringify Boolean values into strings without an explicit mapping.

Future Score support must distinguish selected level from expected value;
OpenAI's score can lie between levels. Do not infer the winner from that scalar.
Future Predicate support must preserve positive-statement probability. Neither
future feature requires Jev's native response shape in the core.

The in-progress
[Jev4Mellea PR #28](https://github.com/SoundBlaster/Jev4Mellea/pull/28)
was inspected at `7185527da2430219c454d33bc792e093b3a14e80`.
It is coordination input, not an implemented or merged Rust dependency.
SwiftDecision's source at `fc28e77976228885b225ca053d08ccbd08c57893`
provides a design reference. Core repository inspected at
`043a0bae6e2834e2a342de1a200a215baed16dd6`; a dependency version/pin is
selected only during implementation after API verification.

## Roadmap and acceptance

The [Choice routing contract proposal](choice-routing-contract.md) expands the
finite evidence states, named validation rules and terminal routing tables for
the first implementation stage.

1. Review this contract, including ID mapping, ties, numerical tolerance,
   failure outcomes, backend capabilities and execution semantics.
2. Implement text Choice with a deterministic fixture backend and named Core
   rules. Exhaustively check finite policy routing states; test invalid
   distributions, missing confidence, refusal and failure paths.
3. Implement RustJev's mapping and migrate SpecificationMetrics with parity
   checks for reports, two classification axes and authority boundaries.
4. Design a separate OpenAI adapter using its native contract; compare adapters
   on identical reviewed cases before sharing thresholds.
5. Add Predicate, Score and multi-question execution only with explicit mapping
   contracts and consumer demand.

No live inference or quality claim follows from this document. Initial success
means typed mapping, deterministic contract checks, visible uncertainty and
unchanged consumer behavior. Model accuracy requires separate evaluation.
