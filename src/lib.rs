#![forbid(unsafe_code)]
//! Runtime-neutral, text-only Choice decisions. No networking or retries.
//! Rules validate structural contracts and caller policy, not semantic truth.
#[cfg(feature = "verification")]
#[doc(hidden)]
pub mod routing;
#[cfg(not(feature = "verification"))]
mod routing;
mod rules;
use specification_core::Specification;

/// A caller-owned option mapping; values never cross the backend boundary.
#[derive(Clone, Debug)]
pub struct Choice<T> {
    pub id: String,
    pub description: String,
    pub value: T,
}
/// One named, ordered text Choice question.
#[derive(Clone, Debug)]
pub struct Request<T> {
    pub question_id: String,
    pub context: String,
    pub instructions: String,
    pub options: Vec<Choice<T>>,
}
/// Allowed fallback causes; operational errors are deliberately absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    ProviderRefusal,
    MissingEvidence,
    PolicyRejected,
    ConflictingEvidence,
}
/// An explicit caller fallback, mapped through the request options.
#[derive(Clone, Debug)]
pub struct Fallback {
    pub option_id: String,
    pub triggers: Vec<Reason>,
}
/// Caller thresholds and absolute normalization/tie tolerance.
#[derive(Clone, Debug)]
pub struct Policy {
    pub min_probability: Option<f64>,
    pub min_confidence: Option<f64>,
    pub epsilon: f64,
    pub fallback: Option<Fallback>,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            min_probability: None,
            min_confidence: None,
            epsilon: 1e-6,
            fallback: None,
        }
    }
}
/// Explicit adapter limits, counted in UTF-8 bytes; no hidden truncation.
#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    pub text_choice: bool,
    pub max_text_bytes: usize,
    pub max_options: usize,
}
/// Adapter confidence representation. Unavailable must be explicitly declared.
#[derive(Clone, Debug)]
pub enum Confidence {
    Present(f64),
    Unavailable,
    Unsupported,
    Contradictory,
}
/// One reconciled answer. Adapters must reject duplicate/wrong-kind answers.
#[derive(Clone, Debug)]
pub struct Prediction {
    pub question_id: String,
    pub selected_id: String,
    pub probabilities: Vec<(String, f64)>,
    pub confidence: Confidence,
}
/// Safe backend categories; provider messages and credentials are excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    RequestValidation,
    PolicyValidation,
    CapabilityValidation,
    OutputValidation,
    Transport,
    Authentication,
    MalformedResponse,
    UnsupportedCapability,
    TimedOut,
}
/// Operational failures an adapter may report; core validation errors cannot be forged here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendFailure {
    Transport,
    Authentication,
    MalformedResponse,
    UnsupportedCapability,
    TimedOut,
}
/// Exactly one terminal backend event; multi-answer parsing belongs to adapters.
#[derive(Clone, Debug)]
pub enum BackendEvent {
    Prediction(Prediction),
    Refusal,
    Failed(BackendFailure),
    Cancelled,
}
/// Provider-safe request view, excluding local application values.
#[derive(Clone, Debug)]
pub struct BackendRequest {
    pub question_id: String,
    pub context: String,
    pub instructions: String,
    pub options: Vec<(String, String)>,
}
/// Synchronous fixture/backend boundary. Does not promise I/O interruption.
pub trait Backend {
    /// Pure local declaration; must not start inference or perform network I/O.
    fn capabilities(&self) -> Capabilities;
    fn invoke(&mut self, request: &BackendRequest) -> BackendEvent;
}
/// Observations sampled before every core transition. Cancellation wins.
#[derive(Clone, Copy, Debug, Default)]
pub struct Observation {
    pub cancelled: bool,
    pub expired: bool,
}
/// Four-valued assessment; skipped checks have no trace entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assessment {
    Satisfied,
    Violated,
    Unknown,
    Conflict,
}
/// Safe, stable rule evidence. No raw input, provider output or secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleTrace {
    pub rule: &'static str,
    pub assessment: Assessment,
}
/// Terminal semantic result, preserving fallback and operational distinctions.
#[derive(Clone, Debug, PartialEq)]
pub enum Decision<T> {
    Accepted(T),
    Abstained(Reason),
    Fallback { value: T, reason: Reason },
    Failed(Failure),
    Cancelled,
}
/// Decision plus evaluated rules and reached lifecycle boundaries.
#[derive(Clone, Debug)]
pub struct Report<T> {
    pub decision: Decision<T>,
    pub rules: Vec<RuleTrace>,
    pub lifecycle: Vec<String>,
    pub invocations: u8,
    pub epsilon: f64,
}
pub(crate) struct Context<'a, T> {
    request: &'a Request<T>,
    policy: &'a Policy,
    capabilities: Capabilities,
    prediction: Option<&'a Prediction>,
}
impl<T> Context<'_, T> {
    fn probability(&self) -> Option<f64> {
        let p = self.prediction?;
        p.probabilities
            .iter()
            .find(|(id, _)| id == &p.selected_id)
            .map(|(_, p)| *p)
    }
}
fn assess<T>(index: usize, c: &Context<'_, T>) -> &'static str {
    use rules::*;
    let satisfied = match index {
        0 => RequestWellFormed.is_satisfied_by(c),
        1 => CallerPolicyWellFormed.is_satisfied_by(c),
        2 => BackendSupportsRequest.is_satisfied_by(c),
        3 => AnswerMatchesRequest.is_satisfied_by(c),
        4 => ProbabilityDistributionValid.is_satisfied_by(c),
        5 => match c.prediction.map(|p| &p.confidence) {
            Some(Confidence::Unsupported) => return "U",
            Some(Confidence::Contradictory) => return "C",
            _ => AdvertisedConfidenceValid.is_satisfied_by(c),
        },
        6 => SelectedOptionConsistent.is_satisfied_by(c),
        7 => {
            if !AcceptanceEvidencePresent.is_satisfied_by(c) {
                return "U";
            }
            true
        }
        8 => AcceptancePolicySatisfied.is_satisfied_by(c),
        _ => unreachable!("internal rule"),
    };
    if satisfied { "S" } else { "V" }
}
fn reason(s: &str) -> Reason {
    match s {
        "provider_refusal" => Reason::ProviderRefusal,
        "missing_evidence" => Reason::MissingEvidence,
        "policy_rejected" => Reason::PolicyRejected,
        _ => Reason::ConflictingEvidence,
    }
}
fn failure(s: &str) -> Failure {
    match s {
        "request_validation" => Failure::RequestValidation,
        "policy_validation" => Failure::PolicyValidation,
        "capability_validation" => Failure::CapabilityValidation,
        "output_validation" => Failure::OutputValidation,
        "transport_error" => Failure::Transport,
        "authentication_error" => Failure::Authentication,
        "unsupported" => Failure::UnsupportedCapability,
        "timed_out" => Failure::TimedOut,
        _ => Failure::MalformedResponse,
    }
}
fn event_name(event: &BackendEvent) -> &'static str {
    match event {
        BackendEvent::Prediction(_) => "prediction",
        BackendEvent::Refusal => "refusal",
        BackendEvent::Cancelled => "cancelled",
        BackendEvent::Failed(BackendFailure::Transport) => "transport_error",
        BackendEvent::Failed(BackendFailure::Authentication) => "authentication_error",
        BackendEvent::Failed(BackendFailure::UnsupportedCapability) => "unsupported",
        BackendEvent::Failed(BackendFailure::TimedOut) => "timed_out",
        BackendEvent::Failed(_) => "malformed_response",
    }
}
/// Validate, invoke once, evaluate and commit. No retry, escalation or live I/O
/// policy is hidden here. Observations cannot interrupt a blocking invoke;
/// they are sampled again after it returns and before terminal commitment.
pub fn decide<T: Clone>(
    request: &Request<T>,
    policy: &Policy,
    backend: &mut impl Backend,
    mut observe: impl FnMut() -> Observation,
) -> Report<T> {
    let capabilities = backend.capabilities();
    let mask = policy
        .fallback
        .as_ref()
        .map(|f| {
            f.triggers.iter().fold(0, |m, r| {
                m | match r {
                    Reason::ProviderRefusal => 1,
                    Reason::MissingEvidence => 2,
                    Reason::PolicyRejected => 4,
                    Reason::ConflictingEvidence => 0,
                }
            })
        })
        .unwrap_or(-1);
    let mut state = routing::State::new(routing::Input {
        plan: vec![],
        confidence: String::new(),
        requires_confidence: policy.min_confidence.is_some(),
        fallback: mask,
    });
    let mut event = None;
    let mut lifecycle = vec![];
    while state.phase != "Done" {
        lifecycle.push(state.phase.clone());
        let observed = observe();
        let mut action = routing::Action {
            cancel: observed.cancelled,
            expire: observed.expired,
            ..Default::default()
        };
        if state.phase == "Await" {
            // The synchronous backend has already returned: its cancellation is
            // observable at this boundary and wins over simultaneous expiry.
            action.cancel |= matches!(event, Some(BackendEvent::Cancelled));
            action.deliver = event_name(event.as_ref().expect("invocation supplies event")).into();
        }
        let context = Context {
            request,
            policy,
            capabilities,
            prediction: match &event {
                Some(BackendEvent::Prediction(p)) => Some(p),
                _ => None,
            },
        };
        let after = state.step(&action, |i| assess(i, &context).into());
        if state.invocations == 0 && after.invocations == 1 {
            let wire = BackendRequest {
                question_id: request.question_id.clone(),
                context: request.context.clone(),
                instructions: request.instructions.clone(),
                options: request
                    .options
                    .iter()
                    .map(|o| (o.id.clone(), o.description.clone()))
                    .collect(),
            };
            event = Some(backend.invoke(&wire));
        }
        state = after;
    }
    lifecycle.push("Done".into());
    let out = state.published.as_ref().expect("Done is published");
    let mapped = |id: &str| {
        request
            .options
            .iter()
            .find(|o| o.id == id)
            .expect("validated mapping")
            .value
            .clone()
    };
    let decision = match out.kind.as_str() {
        "Accepted" => match &event {
            Some(BackendEvent::Prediction(p)) => Decision::Accepted(mapped(&p.selected_id)),
            _ => unreachable!(),
        },
        "Fallback" => Decision::Fallback {
            value: mapped(
                &policy
                    .fallback
                    .as_ref()
                    .expect("validated fallback")
                    .option_id,
            ),
            reason: reason(&out.reason),
        },
        "Abstained" => Decision::Abstained(reason(&out.reason)),
        "Cancelled" => Decision::Cancelled,
        _ => Decision::Failed(failure(&out.reason)),
    };
    let rules = state
        .log
        .iter()
        .map(|(i, a)| RuleTrace {
            rule: rules::IDS[*i],
            assessment: match a.as_str() {
                "S" => Assessment::Satisfied,
                "V" => Assessment::Violated,
                "U" => Assessment::Unknown,
                _ => Assessment::Conflict,
            },
        })
        .collect();
    Report {
        decision,
        rules,
        lifecycle,
        invocations: state.invocations,
        epsilon: policy.epsilon,
    }
}
