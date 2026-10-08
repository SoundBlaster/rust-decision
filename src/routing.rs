//! Finite routing engine. Wire representation is enabled only for verification.
#[cfg_attr(feature = "verification", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub plan: Vec<String>,
    pub confidence: String,
    pub requires_confidence: bool,
    pub fallback: i8,
}
#[cfg_attr(feature = "verification", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub kind: String,
    pub reason: String,
    pub value: String,
}
impl Outcome {
    fn new(kind: &str, reason: &str, value: &str) -> Self {
        Self {
            kind: kind.into(),
            reason: reason.into(),
            value: value.into(),
        }
    }
}
#[cfg_attr(feature = "verification", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub inputs: Input,
    pub phase: String,
    pub invocations: u8,
    pub event: String,
    pub assessed: Vec<String>,
    pub log: Vec<(usize, String)>,
    pub cursor: usize,
    pub cancelled: bool,
    pub expired: bool,
    pub candidate: Option<Outcome>,
    pub published: Option<Outcome>,
}
#[cfg_attr(feature = "verification", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Action {
    pub cancel: bool,
    pub expire: bool,
    pub deliver: String,
}
impl State {
    pub fn new(inputs: Input) -> Self {
        Self {
            inputs,
            phase: "Request".into(),
            invocations: 0,
            event: String::new(),
            assessed: vec!["NE".into(); 9],
            log: vec![],
            cursor: 0,
            cancelled: false,
            expired: false,
            candidate: None,
            published: None,
        }
    }
    fn propose(&mut self, out: Outcome) {
        self.phase = "Commit".into();
        self.candidate = Some(out);
    }
    fn route(&self, reason: &str) -> Outcome {
        let bit = match reason {
            "provider_refusal" => 1,
            "missing_evidence" => 2,
            "policy_rejected" => 4,
            _ => 0,
        };
        if bit != 0 && self.inputs.fallback >= 0 && self.inputs.fallback & bit != 0 {
            Outcome::new("Fallback", reason, "fallback_value")
        } else {
            Outcome::new("Abstained", reason, "")
        }
    }
    fn assess(&mut self, index: usize, evaluate: &mut impl FnMut(usize) -> String) -> String {
        let value = evaluate(index);
        self.assessed[index] = value.clone();
        self.log.push((index, value.clone()));
        value
    }
    /// Deterministic boundary transition. The evaluator is called only for reached rules.
    pub fn step(&self, action: &Action, mut evaluate: impl FnMut(usize) -> String) -> Self {
        if self.phase == "Done" {
            return self.clone();
        }
        let mut s = self.clone();
        s.cancelled |= action.cancel || (s.phase == "Dispatch" && s.event == "cancelled");
        s.expired |= action.expire;
        if s.cancelled || s.expired {
            s.phase = "Done".into();
            s.published = Some(if s.cancelled {
                Outcome::new("Cancelled", "", "")
            } else {
                Outcome::new("Failed", "timed_out", "")
            });
            return s;
        }
        match s.phase.as_str() {
            "Request" | "Policy" | "Capability" => {
                let i = match s.phase.as_str() {
                    "Request" => 0,
                    "Policy" => 1,
                    _ => 2,
                };
                if s.assess(i, &mut evaluate) == "S" {
                    s.phase = ["Policy", "Capability", "Invoke"][i].into();
                } else {
                    s.propose(Outcome::new(
                        "Failed",
                        [
                            "request_validation",
                            "policy_validation",
                            "capability_validation",
                        ][i],
                        "",
                    ));
                }
            }
            "Invoke" => {
                s.invocations = 1;
                s.phase = "Await".into();
            }
            "Await" => {
                if !action.deliver.is_empty() {
                    s.event = action.deliver.clone();
                    s.phase = "Dispatch".into();
                }
            }
            "Dispatch" => match s.event.as_str() {
                "prediction" => {
                    s.phase = "Output".into();
                    s.cursor = 0;
                }
                "refusal" => s.propose(s.route("provider_refusal")),
                reason => {
                    let reason = reason.to_owned();
                    s.propose(Outcome::new("Failed", &reason, ""));
                }
            },
            "Output" => {
                let i = s.cursor + 3;
                if s.assess(i, &mut evaluate) != "S" {
                    s.propose(Outcome::new("Failed", "output_validation", ""));
                } else if s.cursor == 3 {
                    s.phase = "Evidence".into();
                } else {
                    s.cursor += 1;
                }
            }
            "Evidence" => match s.assess(7, &mut evaluate).as_str() {
                "S" => s.phase = "Thresholds".into(),
                "C" => s.propose(s.route("conflicting_evidence")),
                _ => s.propose(s.route("missing_evidence")),
            },
            "Thresholds" => {
                let out = match s.assess(8, &mut evaluate).as_str() {
                    "S" => Outcome::new("Accepted", "", "selected_value"),
                    "V" => s.route("policy_rejected"),
                    "U" => s.route("missing_evidence"),
                    _ => s.route("conflicting_evidence"),
                };
                s.propose(out);
            }
            "Commit" => {
                s.published = s.candidate.clone();
                s.phase = "Done".into();
            }
            _ => unreachable!("internal phase"),
        }
        s
    }
}
