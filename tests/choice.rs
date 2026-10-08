use rust_decision::*;
struct Fixture {
    event: BackendEvent,
    calls: usize,
    supported: bool,
}
impl Backend for Fixture {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            text_choice: self.supported,
            max_text_bytes: 10000,
            max_options: 10,
        }
    }
    fn invoke(&mut self, r: &BackendRequest) -> BackendEvent {
        self.calls += 1;
        assert_eq!(r.options.len(), 2);
        self.event.clone()
    }
}
fn request() -> Request<u8> {
    Request {
        question_id: "q".into(),
        context: "context".into(),
        instructions: "choose".into(),
        options: vec![
            Choice {
                id: "a".into(),
                description: "A".into(),
                value: 1,
            },
            Choice {
                id: "b".into(),
                description: "B".into(),
                value: 2,
            },
        ],
    }
}
fn prediction() -> Prediction {
    Prediction {
        question_id: "q".into(),
        selected_id: "a".into(),
        probabilities: vec![("a".into(), 0.8), ("b".into(), 0.2)],
        confidence: Confidence::Present(0.9),
    }
}
fn run(p: Prediction, policy: Policy) -> Report<u8> {
    decide(
        &request(),
        &policy,
        &mut Fixture {
            event: BackendEvent::Prediction(p),
            calls: 0,
            supported: true,
        },
        Observation::default,
    )
}
#[test]
fn accepts_mapped_value_and_records_nine_rules() {
    let r = run(
        prediction(),
        Policy {
            min_probability: Some(0.7),
            ..Default::default()
        },
    );
    assert_eq!(r.decision, Decision::Accepted(1));
    assert_eq!(r.rules.len(), 9);
    assert_eq!(r.invocations, 1);
}
#[test]
fn invalid_present_confidence_is_never_missing_evidence() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
        for min_probability in [None, Some(0.7)] {
            let mut p = prediction();
            p.confidence = Confidence::Present(value);
            let r = run(
                p,
                Policy {
                    min_probability,
                    ..Default::default()
                },
            );
            assert_eq!(r.decision, Decision::Failed(Failure::OutputValidation));
            assert_eq!(r.rules.last().unwrap().rule, "AdvertisedConfidenceValid");
        }
    }
}
#[test]
fn unavailable_confidence_depends_on_required_evidence() {
    let mut p = prediction();
    p.confidence = Confidence::Unavailable;
    assert_eq!(
        run(p.clone(), Policy::default()).decision,
        Decision::Accepted(1)
    );
    let r = run(
        p,
        Policy {
            min_confidence: Some(0.5),
            ..Default::default()
        },
    );
    assert_eq!(r.decision, Decision::Abstained(Reason::MissingEvidence));
    assert_eq!(r.rules.len(), 8);
}
#[test]
fn rejects_invalid_distributions_and_identity_without_normalization() {
    for probabilities in [
        vec![("a".into(), 0.7), ("b".into(), 0.2)],
        vec![("a".into(), f64::NAN), ("b".into(), 0.2)],
        vec![("a".into(), 1.1), ("b".into(), -0.1)],
        vec![("a".into(), 0.8), ("a".into(), 0.2)],
        vec![("a".into(), 0.8), ("extra".into(), 0.2)],
    ] {
        let mut p = prediction();
        p.probabilities = probabilities;
        assert_eq!(
            run(p, Policy::default()).decision,
            Decision::Failed(Failure::OutputValidation)
        );
    }
    for wrong_question in [true, false] {
        let mut p = prediction();
        if wrong_question {
            p.question_id = "wrong".into();
        } else {
            p.selected_id = "unknown".into();
        }
        assert_eq!(
            run(p, Policy::default()).decision,
            Decision::Failed(Failure::OutputValidation)
        );
    }
}
#[test]
fn preserves_tied_selection_and_probability_storage_permutation() {
    let mut p = prediction();
    p.selected_id = "b".into();
    p.probabilities = vec![("a".into(), 0.5), ("b".into(), 0.5)];
    assert_eq!(
        run(p.clone(), Policy::default()).decision,
        Decision::Accepted(2)
    );
    p.probabilities.reverse();
    assert_eq!(run(p, Policy::default()).decision, Decision::Accepted(2));
}
#[test]
fn raising_or_adding_thresholds_cannot_rescue_rejection() {
    for threshold in [0.0, 0.5, 0.8, 0.800001, 0.9, 1.0] {
        let r = run(
            prediction(),
            Policy {
                min_probability: Some(threshold),
                ..Default::default()
            },
        );
        assert_eq!(
            matches!(r.decision, Decision::Accepted(_)),
            threshold <= 0.8
        );
        if threshold > 0.8 {
            assert_eq!(
                run(
                    prediction(),
                    Policy {
                        min_probability: Some(threshold),
                        min_confidence: Some(0.1),
                        ..Default::default()
                    }
                )
                .decision,
                Decision::Abstained(Reason::PolicyRejected)
            );
        }
    }
}
#[test]
fn invalid_request_policy_capability_never_invokes() {
    for i in 0..4 {
        let mut r = request();
        let mut p = Policy::default();
        let mut b = Fixture {
            event: BackendEvent::Prediction(prediction()),
            calls: 0,
            supported: true,
        };
        let expected = match i {
            0 => {
                r.options[1].id = "a".into();
                Failure::RequestValidation
            }
            1 => {
                p.epsilon = f64::NAN;
                Failure::PolicyValidation
            }
            2 => {
                p.fallback = Some(Fallback {
                    option_id: "missing".into(),
                    triggers: vec![Reason::ProviderRefusal],
                });
                Failure::PolicyValidation
            }
            _ => {
                b.supported = false;
                Failure::CapabilityValidation
            }
        };
        assert_eq!(
            decide(&r, &p, &mut b, Observation::default).decision,
            Decision::Failed(expected)
        );
        assert_eq!(b.calls, 0);
    }
}
#[test]
fn fallback_does_not_hide_operational_failures() {
    let p = Policy {
        fallback: Some(Fallback {
            option_id: "b".into(),
            triggers: vec![
                Reason::ProviderRefusal,
                Reason::MissingEvidence,
                Reason::PolicyRejected,
            ],
        }),
        ..Default::default()
    };
    for (event, expected) in [
        (
            BackendEvent::Refusal,
            Decision::Fallback {
                value: 2,
                reason: Reason::ProviderRefusal,
            },
        ),
        (
            BackendEvent::Failed(BackendFailure::Authentication),
            Decision::Failed(Failure::Authentication),
        ),
        (
            BackendEvent::Failed(BackendFailure::Transport),
            Decision::Failed(Failure::Transport),
        ),
        (BackendEvent::Cancelled, Decision::Cancelled),
    ] {
        let mut b = Fixture {
            event,
            calls: 0,
            supported: true,
        };
        let r = decide(&request(), &p, &mut b, Observation::default);
        assert_eq!(r.decision, expected);
        assert_eq!(b.calls, 1);
    }
    assert_eq!(
        run(
            prediction(),
            Policy {
                min_probability: Some(0.9),
                ..p
            }
        )
        .decision,
        Decision::Fallback {
            value: 2,
            reason: Reason::PolicyRejected
        }
    );
}
#[test]
fn cancellation_wins_at_every_boundary_including_commit() {
    let full = run(prediction(), Policy::default());
    for stop in 0..full.lifecycle.len() - 1 {
        let mut n = 0;
        let mut b = Fixture {
            event: BackendEvent::Prediction(prediction()),
            calls: 0,
            supported: true,
        };
        let r = decide(&request(), &Policy::default(), &mut b, || {
            let observed = Observation {
                cancelled: n == stop,
                expired: n == stop,
            };
            n += 1;
            observed
        });
        assert_eq!(r.decision, Decision::Cancelled);
        assert!(b.calls <= 1);
    }
}
#[test]
fn expiry_after_blocking_invoke_is_observed_before_acceptance() {
    let mut n = 0;
    let mut b = Fixture {
        event: BackendEvent::Prediction(prediction()),
        calls: 0,
        supported: true,
    };
    let r = decide(&request(), &Policy::default(), &mut b, || {
        n += 1;
        Observation {
            expired: n == 5,
            cancelled: false,
        }
    });
    assert_eq!(r.decision, Decision::Failed(Failure::TimedOut));
    assert_eq!(b.calls, 1);
    assert_eq!(r.rules.len(), 3);
}

#[test]
fn request_and_probability_permutations_preserve_local_mapping() {
    let mut r = request();
    r.options.reverse();
    let mut p = prediction();
    p.probabilities.reverse();
    let mut b = Fixture {
        event: BackendEvent::Prediction(p),
        calls: 0,
        supported: true,
    };
    assert_eq!(
        decide(&r, &Policy::default(), &mut b, Observation::default).decision,
        Decision::Accepted(1)
    );
}
#[test]
fn malformed_thresholds_and_fallback_triggers_fail_before_backend() {
    for threshold in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        for confidence in [false, true] {
            let p = if confidence {
                Policy {
                    min_confidence: Some(threshold),
                    ..Default::default()
                }
            } else {
                Policy {
                    min_probability: Some(threshold),
                    ..Default::default()
                }
            };
            let mut b = Fixture {
                event: BackendEvent::Refusal,
                calls: 0,
                supported: true,
            };
            assert_eq!(
                decide(&request(), &p, &mut b, Observation::default).decision,
                Decision::Failed(Failure::PolicyValidation)
            );
            assert_eq!(b.calls, 0);
        }
    }
    for triggers in [
        vec![Reason::ConflictingEvidence],
        vec![Reason::ProviderRefusal, Reason::ProviderRefusal],
    ] {
        let p = Policy {
            fallback: Some(Fallback {
                option_id: "b".into(),
                triggers,
            }),
            ..Default::default()
        };
        assert_eq!(
            run(prediction(), p).decision,
            Decision::Failed(Failure::PolicyValidation)
        );
    }
}
#[test]
fn epsilon_never_relaxes_range_or_acceptance_threshold() {
    let policy = Policy {
        epsilon: 1e-3,
        ..Default::default()
    };
    let mut p = prediction();
    p.probabilities[1].1 = 0.2005;
    assert_eq!(
        run(p.clone(), policy.clone()).decision,
        Decision::Accepted(1)
    );
    assert_eq!(
        run(
            p,
            Policy {
                min_probability: Some(0.8001),
                ..policy.clone()
            }
        )
        .decision,
        Decision::Abstained(Reason::PolicyRejected)
    );
    let mut p = prediction();
    p.probabilities = vec![("a".into(), 1.0001), ("b".into(), 0.0)];
    assert_eq!(
        run(p, policy).decision,
        Decision::Failed(Failure::OutputValidation)
    );
}
#[test]
fn nonmaximal_selection_and_unsupported_confidence_fail_structurally() {
    let mut p = prediction();
    p.selected_id = "b".into();
    assert_eq!(
        run(p, Policy::default()).decision,
        Decision::Failed(Failure::OutputValidation)
    );
    for confidence in [Confidence::Unsupported, Confidence::Contradictory] {
        let mut p = prediction();
        p.confidence = confidence;
        assert_eq!(
            run(p, Policy::default()).decision,
            Decision::Failed(Failure::OutputValidation)
        );
    }
}

#[test]
fn returned_backend_cancellation_wins_over_simultaneous_expiry() {
    let mut n = 0;
    let mut b = Fixture {
        event: BackendEvent::Cancelled,
        calls: 0,
        supported: true,
    };
    let r = decide(&request(), &Policy::default(), &mut b, || {
        n += 1;
        Observation {
            expired: n == 5,
            cancelled: false,
        }
    });
    assert_eq!(r.decision, Decision::Cancelled);
    assert_eq!(b.calls, 1);
}

const THREE_OPTION_PERMUTATIONS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

fn assert_distribution_permutations(values: [f64; 3], epsilon: f64, expected: Decision<u8>) {
    struct PermutationFixture {
        prediction: Prediction,
        request_order: Vec<String>,
    }
    impl Backend for PermutationFixture {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                text_choice: true,
                max_text_bytes: 10000,
                max_options: 10,
            }
        }
        fn invoke(&mut self, request: &BackendRequest) -> BackendEvent {
            // The numerical fix must not reorder the caller's wire request.
            assert_eq!(
                request
                    .options
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>(),
                self.request_order
            );
            BackendEvent::Prediction(self.prediction.clone())
        }
    }
    let mut original = request();
    original.options.push(Choice {
        id: "c".into(),
        description: "C".into(),
        value: 3,
    });
    let ids = ["a", "b", "c"];
    let policy = Policy {
        epsilon,
        ..Default::default()
    };
    let mut baseline_rules = None;
    for request_order in THREE_OPTION_PERMUTATIONS {
        for probability_order in THREE_OPTION_PERMUTATIONS {
            let mut request = original.clone();
            request.options = request_order
                .iter()
                .map(|&i| original.options[i].clone())
                .collect();
            let mut backend = PermutationFixture {
                prediction: Prediction {
                    question_id: "q".into(),
                    selected_id: "c".into(),
                    probabilities: probability_order
                        .iter()
                        .map(|&i| (ids[i].into(), values[i]))
                        .collect(),
                    confidence: Confidence::Unavailable,
                },
                request_order: request.options.iter().map(|o| o.id.clone()).collect(),
            };
            let report = decide(&request, &policy, &mut backend, Observation::default);
            assert_eq!(
                report.decision, expected,
                "request={request_order:?}, probabilities={probability_order:?}, epsilon={epsilon}"
            );
            assert_eq!(report.invocations, 1);
            if let Some(rules) = &baseline_rules {
                assert_eq!(&report.rules, rules);
            } else {
                baseline_rules = Some(report.rules);
            }
        }
    }
}

#[test]
fn three_option_permutations_are_invariant_at_zero_tolerance() {
    assert_distribution_permutations([0.1, 0.2, 0.7], 0.0, Decision::Accepted(3));
}

#[test]
fn three_option_permutations_are_invariant_at_default_tolerance_boundary() {
    assert_distribution_permutations(
        [0.1, 0.2, 0.700001],
        Policy::default().epsilon,
        Decision::Accepted(3),
    );
}

#[test]
fn permutation_stability_does_not_relax_normalization_tolerance() {
    for (values, epsilon) in [
        ([0.1, 0.2, 0.6999999999999998], 0.0),
        ([0.1, 0.2, 0.700002], Policy::default().epsilon),
    ] {
        assert_distribution_permutations(
            values,
            epsilon,
            Decision::Failed(Failure::OutputValidation),
        );
    }
}
