use super::*;
use crate::{Assessment, Confidence, Decision, Observation, Report, RuleTrace, routing};
use specification_core::Specification;
fn assess(index: usize, c: &ScalarContext<'_>) -> &'static str {
    use super::rules::*;
    let valid = match index {
        0 => ScalarRequestWellFormed.is_satisfied_by(c),
        1 => ScalarPolicyWellFormed.is_satisfied_by(c),
        2 => ScalarBackendSupportsRequest.is_satisfied_by(c),
        3 => ScalarAnswerMatchesRequest.is_satisfied_by(c),
        4 => ScalarProbabilitiesValid.is_satisfied_by(c),
        5 => match c.confidence() {
            Some(Confidence::Unsupported) => return "U",
            Some(Confidence::Contradictory) => return "C",
            _ => ScalarConfidenceValid.is_satisfied_by(c),
        },
        6 => ScalarValueConsistent.is_satisfied_by(c),
        7 => {
            if ScalarEvidencePresent.is_satisfied_by(c) {
                true
            } else {
                return "U";
            }
        }
        8 => ScalarPolicySatisfied.is_satisfied_by(c),
        _ => unreachable!("internal rule"),
    };
    if valid { "S" } else { "V" }
}
fn event_name(event: &ScalarEvent) -> &'static str {
    match event {
        ScalarEvent::Prediction(_) => "prediction",
        ScalarEvent::Refusal => "refusal",
        ScalarEvent::Cancelled => "cancelled",
        ScalarEvent::Failed(f) => crate::event_name(&crate::BackendEvent::Failed(*f)),
    }
}
fn fallback(policy: &ScalarPolicy) -> Option<(ScalarValue, &[Reason])> {
    match policy {
        ScalarPolicy::Predicate(p) => p
            .fallback
            .as_ref()
            .map(|f| (ScalarValue::Predicate(f.value), f.triggers.as_slice())),
        ScalarPolicy::Score(p) => p
            .fallback
            .as_ref()
            .map(|f| (ScalarValue::Score(f.value), f.triggers.as_slice())),
    }
}
/// One native predicate or rubric score through the existing finite lifecycle.
/// Wrong-kind answers fail validation; operational errors never select fallback.
/// Observation/cancellation semantics are the same as Choice's synchronous API.
pub fn decide_scalar(
    request: &ScalarRequest,
    policy: &ScalarPolicy,
    backend: &mut impl ScalarBackend,
    mut observe: impl FnMut() -> Observation,
) -> Report<ScalarValue> {
    let capabilities = backend.scalar_capabilities();
    let fallback = fallback(policy);
    let mask = fallback
        .map(|(_, triggers)| {
            triggers.iter().fold(0, |m, r| {
                m | match r {
                    Reason::ProviderRefusal => 1,
                    Reason::MissingEvidence => 2,
                    Reason::PolicyRejected => 4,
                    Reason::ConflictingEvidence => 0,
                }
            })
        })
        .unwrap_or(-1);
    let requires_confidence = matches!(policy,ScalarPolicy::Score(p) if p.min_confidence.is_some());
    let epsilon = match policy {
        ScalarPolicy::Predicate(_) => 0.0,
        ScalarPolicy::Score(p) => p.epsilon,
    };
    let mut state = routing::State::new(routing::Input {
        plan: vec![],
        confidence: String::new(),
        requires_confidence,
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
            action.cancel |= matches!(event, Some(ScalarEvent::Cancelled));
            action.deliver = event_name(event.as_ref().expect("invocation supplies event")).into();
        }
        let context = ScalarContext {
            request,
            policy,
            capabilities,
            prediction: match &event {
                Some(ScalarEvent::Prediction(p)) => Some(p),
                _ => None,
            },
        };
        let after = state.step(&action, |i| assess(i, &context).into());
        if state.invocations == 0 && after.invocations == 1 {
            event = Some(backend.invoke_scalar(request));
        }
        state = after;
    }
    lifecycle.push("Done".into());
    let outcome = state.published.as_ref().expect("Done is published");
    let decision = match outcome.kind.as_str() {
        "Accepted" => Decision::Accepted(match (&event, policy) {
            (
                Some(ScalarEvent::Prediction(ScalarPrediction::Predicate {
                    probability_true, ..
                })),
                ScalarPolicy::Predicate(p),
            ) => ScalarValue::Predicate(*probability_true >= p.accept_at),
            (
                Some(ScalarEvent::Prediction(ScalarPrediction::Score { score, .. })),
                ScalarPolicy::Score(_),
            ) => ScalarValue::Score(*score),
            _ => unreachable!("validated scalar answer"),
        }),
        "Fallback" => Decision::Fallback {
            value: fallback.expect("validated fallback").0,
            reason: crate::reason(&outcome.reason),
        },
        "Abstained" => Decision::Abstained(crate::reason(&outcome.reason)),
        "Cancelled" => Decision::Cancelled,
        _ => Decision::Failed(crate::failure(&outcome.reason)),
    };
    let rules = state
        .log
        .iter()
        .map(|(i, a)| RuleTrace {
            rule: super::rules::IDS[*i],
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
        epsilon,
    }
}
fn map_report<T>(r: Report<ScalarValue>, map: impl Fn(ScalarValue) -> T) -> Report<T> {
    Report {
        decision: match r.decision {
            Decision::Accepted(v) => Decision::Accepted(map(v)),
            Decision::Fallback { value, reason } => Decision::Fallback {
                value: map(value),
                reason,
            },
            Decision::Abstained(r) => Decision::Abstained(r),
            Decision::Failed(f) => Decision::Failed(f),
            Decision::Cancelled => Decision::Cancelled,
        },
        rules: r.rules,
        lifecycle: r.lifecycle,
        invocations: r.invocations,
        epsilon: r.epsilon,
    }
}
pub fn decide_predicate(
    request: &PredicateRequest,
    policy: &PredicatePolicy,
    backend: &mut impl ScalarBackend,
    observe: impl FnMut() -> Observation,
) -> Report<bool> {
    map_report(
        decide_scalar(
            &ScalarRequest::Predicate(request.clone()),
            &ScalarPolicy::Predicate(policy.clone()),
            backend,
            observe,
        ),
        |v| match v {
            ScalarValue::Predicate(v) => v,
            _ => unreachable!("typed predicate request"),
        },
    )
}
pub fn decide_score(
    request: &ScoreRequest,
    policy: &ScorePolicy,
    backend: &mut impl ScalarBackend,
    observe: impl FnMut() -> Observation,
) -> Report<f64> {
    map_report(
        decide_scalar(
            &ScalarRequest::Score(request.clone()),
            &ScalarPolicy::Score(policy.clone()),
            backend,
            observe,
        ),
        |v| match v {
            ScalarValue::Score(v) => v,
            _ => unreachable!("typed score request"),
        },
    )
}
