use rust_decision::*;
struct Fixture {
    event: ScalarEvent,
    calls: usize,
    caps: ScalarCapabilities,
}
impl ScalarBackend for Fixture {
    fn scalar_capabilities(&self) -> ScalarCapabilities {
        self.caps
    }
    fn invoke_scalar(&mut self, _: &ScalarRequest) -> ScalarEvent {
        self.calls += 1;
        self.event.clone()
    }
}
fn backend(event: ScalarEvent) -> Fixture {
    Fixture {
        event,
        calls: 0,
        caps: ScalarCapabilities {
            predicate: true,
            score: true,
            max_text_bytes: 4096,
            max_score_levels: 255,
        },
    }
}
fn predicate() -> PredicateRequest {
    PredicateRequest {
        question_id: "q".into(),
        context: "text".into(),
        instructions: "Is it a greeting?".into(),
        true_description: Some("A greeting".into()),
        false_description: None,
    }
}
fn pred(v: f64) -> ScalarEvent {
    ScalarEvent::Prediction(ScalarPrediction::Predicate {
        question_id: "q".into(),
        probability_true: v,
    })
}
fn score() -> ScoreRequest {
    ScoreRequest {
        question_id: "q".into(),
        context: "text".into(),
        instructions: "Rate urgency".into(),
        levels: vec!["low".into(), "medium".into(), "high".into()],
    }
}
fn scored() -> ScalarEvent {
    ScalarEvent::Prediction(ScalarPrediction::Score {
        question_id: "q".into(),
        score: 1.7,
        probabilities: vec![(2, 0.8), (0, 0.1), (1, 0.1)],
        confidence: Confidence::Present(0.9),
    })
}
fn p_run(v: f64) -> Report<bool> {
    decide_predicate(
        &predicate(),
        &PredicatePolicy::default(),
        &mut backend(pred(v)),
        Observation::default,
    )
}
fn s_run(e: ScalarEvent, p: ScorePolicy) -> Report<f64> {
    decide_score(&score(), &p, &mut backend(e), Observation::default)
}
#[test]
fn predicate_thresholds_and_uncertainty() {
    for (p, expected) in [(0.0, false), (0.1, false), (0.9, true), (1.0, true)] {
        let r = p_run(p);
        assert_eq!(r.decision, Decision::Accepted(expected));
        assert_eq!(r.rules.len(), 9);
        assert_eq!(r.invocations, 1);
    }
    for p in [0.10001, 0.5, 0.89999] {
        assert_eq!(
            p_run(p).decision,
            Decision::Abstained(Reason::PolicyRejected)
        );
    }
}
#[test]
fn native_predicate_probability_is_validated_without_confidence() {
    for v in [-0.01, 1.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let r = p_run(v);
        assert_eq!(r.decision, Decision::Failed(Failure::OutputValidation));
        assert_eq!(r.rules.last().unwrap().rule, "ScalarProbabilitiesValid");
    }
}
#[test]
fn predicate_policy_and_requests_fail_before_invocation() {
    for (reject_at, accept_at) in [
        (0.5, 0.5),
        (0.8, 0.2),
        (-0.1, 0.9),
        (0.1, 1.1),
        (f64::NAN, 0.9),
    ] {
        let mut b = backend(pred(0.99));
        let r = decide_predicate(
            &predicate(),
            &PredicatePolicy {
                reject_at,
                accept_at,
                ..Default::default()
            },
            &mut b,
            Observation::default,
        );
        assert_eq!(r.decision, Decision::Failed(Failure::PolicyValidation));
        assert_eq!(b.calls, 0);
    }
    let mut r = predicate();
    r.true_description = Some(" ".into());
    let mut b = backend(pred(0.99));
    assert_eq!(
        decide_predicate(
            &r,
            &PredicatePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::RequestValidation)
    );
    assert_eq!(b.calls, 0);
}
#[test]
fn scalar_kind_and_question_identity_are_checked() {
    let mut b = backend(scored());
    assert_eq!(
        decide_predicate(
            &predicate(),
            &PredicatePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::OutputValidation)
    );
    let mut b = backend(ScalarEvent::Prediction(ScalarPrediction::Predicate {
        question_id: "wrong".into(),
        probability_true: 0.99,
    }));
    assert_eq!(
        decide_predicate(
            &predicate(),
            &PredicatePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::OutputValidation)
    );
    let mut b = backend(pred(0.99));
    let r = decide_scalar(
        &ScalarRequest::Predicate(predicate()),
        &ScalarPolicy::Score(ScorePolicy::default()),
        &mut b,
        Observation::default,
    );
    assert_eq!(r.decision, Decision::Failed(Failure::PolicyValidation));
    assert_eq!(b.calls, 0);
    assert_eq!(
        s_run(pred(0.99), ScorePolicy::default()).decision,
        Decision::Failed(Failure::OutputValidation)
    );
}
#[test]
fn expected_score_and_thresholds_use_native_numeric_value() {
    let r = s_run(scored(), ScorePolicy::default());
    assert_eq!(r.decision, Decision::Accepted(1.7));
    assert_eq!(r.rules.len(), 9);
    for p in [
        ScorePolicy {
            min_score: Some(1.8),
            ..Default::default()
        },
        ScorePolicy {
            max_score: Some(1.6),
            ..Default::default()
        },
        ScorePolicy {
            min_confidence: Some(0.95),
            ..Default::default()
        },
    ] {
        assert_eq!(
            s_run(scored(), p).decision,
            Decision::Abstained(Reason::PolicyRejected)
        );
    }
    let p = ScorePolicy {
        min_score: Some(1.7),
        max_score: Some(1.7),
        min_confidence: Some(0.9),
        ..Default::default()
    };
    assert_eq!(s_run(scored(), p).decision, Decision::Accepted(1.7));
}
#[test]
fn score_range_and_expected_value_must_agree() {
    for v in [-0.1, 2.1, 1.8, f64::NAN, f64::INFINITY] {
        let mut e = scored();
        if let ScalarEvent::Prediction(ScalarPrediction::Score { score, .. }) = &mut e {
            *score = v;
        }
        let r = s_run(e, ScorePolicy::default());
        assert_eq!(r.decision, Decision::Failed(Failure::OutputValidation));
        assert_eq!(r.rules.last().unwrap().rule, "ScalarValueConsistent");
    }
}
#[test]
fn score_distribution_has_exact_unique_level_coverage_and_normalization() {
    for probs in [
        vec![(0, 0.1), (1, 0.1)],
        vec![(0, 0.1), (0, 0.1), (2, 0.8)],
        vec![(0, 0.1), (1, 0.1), (3, 0.8)],
        vec![(0, -0.1), (1, 0.1), (2, 1.0)],
        vec![(0, 0.1), (1, 0.1), (2, 0.9)],
        vec![(0, f64::NAN), (1, 0.1), (2, 0.9)],
    ] {
        let mut e = scored();
        if let ScalarEvent::Prediction(ScalarPrediction::Score { probabilities, .. }) = &mut e {
            *probabilities = probs;
        }
        let r = s_run(e, ScorePolicy::default());
        assert_eq!(r.decision, Decision::Failed(Failure::OutputValidation));
        assert_eq!(r.rules.last().unwrap().rule, "ScalarProbabilitiesValid");
    }
}
#[test]
fn score_confidence_preserves_unavailability_and_invalid_present_values() {
    for confidence in [
        Confidence::Present(-0.1),
        Confidence::Present(1.1),
        Confidence::Present(f64::NAN),
        Confidence::Unsupported,
        Confidence::Contradictory,
    ] {
        let mut e = scored();
        if let ScalarEvent::Prediction(ScalarPrediction::Score { confidence: c, .. }) = &mut e {
            *c = confidence;
        }
        let r = s_run(e, ScorePolicy::default());
        assert_eq!(r.decision, Decision::Failed(Failure::OutputValidation));
        assert_eq!(r.rules.last().unwrap().rule, "ScalarConfidenceValid");
    }
    let mut e = scored();
    if let ScalarEvent::Prediction(ScalarPrediction::Score { confidence, .. }) = &mut e {
        *confidence = Confidence::Unavailable;
    }
    assert_eq!(
        s_run(e.clone(), ScorePolicy::default()).decision,
        Decision::Accepted(1.7)
    );
    assert_eq!(
        s_run(
            e,
            ScorePolicy {
                min_confidence: Some(0.8),
                ..Default::default()
            }
        )
        .decision,
        Decision::Abstained(Reason::MissingEvidence)
    );
}
#[test]
fn score_policy_and_empty_rubric_are_rejected_without_call() {
    for p in [
        ScorePolicy {
            min_score: Some(3.0),
            ..Default::default()
        },
        ScorePolicy {
            min_score: Some(1.5),
            max_score: Some(1.0),
            ..Default::default()
        },
        ScorePolicy {
            epsilon: -1.0,
            ..Default::default()
        },
        ScorePolicy {
            min_confidence: Some(f64::NAN),
            ..Default::default()
        },
        ScorePolicy {
            fallback: Some(ValueFallback {
                value: 3.0,
                triggers: vec![Reason::PolicyRejected],
            }),
            ..Default::default()
        },
    ] {
        let mut b = backend(scored());
        let r = decide_score(&score(), &p, &mut b, Observation::default);
        assert_eq!(r.decision, Decision::Failed(Failure::PolicyValidation));
        assert_eq!(b.calls, 0);
    }
    let mut request = score();
    request.levels.clear();
    let mut b = backend(scored());
    assert_eq!(
        decide_score(
            &request,
            &ScorePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::RequestValidation)
    );
    assert_eq!(b.calls, 0);
}
#[test]
fn semantic_fallback_and_operational_failures_remain_distinct() {
    let p = PredicatePolicy {
        fallback: Some(ValueFallback {
            value: false,
            triggers: vec![Reason::PolicyRejected, Reason::ProviderRefusal],
        }),
        ..Default::default()
    };
    for (e, reason) in [
        (pred(0.5), Reason::PolicyRejected),
        (ScalarEvent::Refusal, Reason::ProviderRefusal),
    ] {
        let r = decide_predicate(&predicate(), &p, &mut backend(e), Observation::default);
        assert_eq!(
            r.decision,
            Decision::Fallback {
                value: false,
                reason
            }
        );
    }
    for (e, expected) in [
        (BackendFailure::Authentication, Failure::Authentication),
        (BackendFailure::Transport, Failure::Transport),
        (BackendFailure::TimedOut, Failure::TimedOut),
        (
            BackendFailure::MalformedResponse,
            Failure::MalformedResponse,
        ),
    ] {
        assert_eq!(
            decide_predicate(
                &predicate(),
                &p,
                &mut backend(ScalarEvent::Failed(e)),
                Observation::default
            )
            .decision,
            Decision::Failed(expected)
        );
    }
    let p = ScorePolicy {
        min_score: Some(1.8),
        fallback: Some(ValueFallback {
            value: 0.0,
            triggers: vec![Reason::PolicyRejected],
        }),
        ..Default::default()
    };
    assert_eq!(
        s_run(scored(), p).decision,
        Decision::Fallback {
            value: 0.0,
            reason: Reason::PolicyRejected
        }
    );
}
#[test]
fn cancellation_and_expiry_win_at_boundaries() {
    let mut b = backend(pred(1.0));
    let r = decide_predicate(&predicate(), &PredicatePolicy::default(), &mut b, || {
        Observation {
            cancelled: true,
            expired: true,
        }
    });
    assert_eq!(r.decision, Decision::Cancelled);
    assert_eq!(b.calls, 0);
    let mut observed = 0;
    let mut b = backend(scored());
    let r = decide_score(&score(), &ScorePolicy::default(), &mut b, || {
        observed += 1;
        Observation {
            cancelled: observed >= 5,
            expired: false,
        }
    });
    assert_eq!(r.decision, Decision::Cancelled);
    assert_eq!(b.calls, 1);
    assert_eq!(
        decide_score(
            &score(),
            &ScorePolicy::default(),
            &mut backend(ScalarEvent::Cancelled),
            Observation::default
        )
        .decision,
        Decision::Cancelled
    );
    let r = decide_predicate(
        &predicate(),
        &PredicatePolicy::default(),
        &mut backend(pred(1.0)),
        || Observation {
            cancelled: false,
            expired: true,
        },
    );
    assert_eq!(r.decision, Decision::Failed(Failure::TimedOut));
    assert_eq!(r.invocations, 0);
}
#[test]
fn scalar_capability_limits_are_preflight_rules() {
    let mut b = backend(pred(0.99));
    b.caps.predicate = false;
    assert_eq!(
        decide_predicate(
            &predicate(),
            &PredicatePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::CapabilityValidation)
    );
    assert_eq!(b.calls, 0);
    let mut b = backend(scored());
    b.caps.max_score_levels = 2;
    assert_eq!(
        decide_score(
            &score(),
            &ScorePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::CapabilityValidation)
    );
    assert_eq!(b.calls, 0);
    let mut b = backend(scored());
    b.caps.max_text_bytes = 1;
    assert_eq!(
        decide_score(
            &score(),
            &ScorePolicy::default(),
            &mut b,
            Observation::default
        )
        .decision,
        Decision::Failed(Failure::CapabilityValidation)
    );
    assert_eq!(b.calls, 0);
}
#[test]
fn one_level_score_is_valid_and_probability_order_does_not_change_decision() {
    let r = ScoreRequest {
        levels: vec!["only".into()],
        ..score()
    };
    let e = ScalarEvent::Prediction(ScalarPrediction::Score {
        question_id: "q".into(),
        score: 0.0,
        probabilities: vec![(0, 1.0)],
        confidence: Confidence::Present(1.0),
    });
    assert_eq!(
        decide_score(
            &r,
            &ScorePolicy::default(),
            &mut backend(e),
            Observation::default
        )
        .decision,
        Decision::Accepted(0.0)
    );
    for probs in [
        vec![(0, 0.125), (1, 0.125), (2, 0.75)],
        vec![(2, 0.75), (0, 0.125), (1, 0.125)],
        vec![(1, 0.125), (2, 0.75), (0, 0.125)],
    ] {
        let mut e = scored();
        if let ScalarEvent::Prediction(ScalarPrediction::Score {
            probabilities,
            score,
            ..
        }) = &mut e
        {
            *probabilities = probs;
            *score = 1.625;
        }
        assert_eq!(
            s_run(
                e,
                ScorePolicy {
                    epsilon: 0.0,
                    ..Default::default()
                }
            )
            .decision,
            Decision::Accepted(1.625)
        );
    }
}
