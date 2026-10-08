//! Provider-neutral binary predicates and ordered rubric scores.
mod rules;
mod runner;
use crate::{BackendFailure, Confidence, Reason};
pub use runner::{decide_predicate, decide_scalar, decide_score};

/// A yes/no question. Native probability is not a separate confidence estimate.
#[derive(Clone, Debug)]
pub struct PredicateRequest {
    pub question_id: String,
    pub context: String,
    pub instructions: String,
    pub true_description: Option<String>,
    pub false_description: Option<String>,
}
/// Position in this rubric defines the numeric level, starting at zero.
#[derive(Clone, Debug)]
pub struct ScoreRequest {
    pub question_id: String,
    pub context: String,
    pub instructions: String,
    pub levels: Vec<String>,
}
#[derive(Clone, Debug)]
pub enum ScalarRequest {
    Predicate(PredicateRequest),
    Score(ScoreRequest),
}
impl ScalarRequest {
    pub fn question_id(&self) -> &str {
        match self {
            Self::Predicate(r) => &r.question_id,
            Self::Score(r) => &r.question_id,
        }
    }
    pub fn context(&self) -> &str {
        match self {
            Self::Predicate(r) => &r.context,
            Self::Score(r) => &r.context,
        }
    }
    pub fn instructions(&self) -> &str {
        match self {
            Self::Predicate(r) => &r.instructions,
            Self::Score(r) => &r.instructions,
        }
    }
    pub fn text_bytes(&self) -> Option<usize> {
        let total = self
            .question_id()
            .len()
            .checked_add(self.context().len())?
            .checked_add(self.instructions().len())?;
        match self {
            Self::Predicate(r) => [
                r.true_description.as_deref(),
                r.false_description.as_deref(),
            ]
            .into_iter()
            .flatten()
            .try_fold(total, |n, s| n.checked_add(s.len())),
            Self::Score(r) => r
                .levels
                .iter()
                .try_fold(total, |n, s| n.checked_add(s.len())),
        }
    }
}
/// Only explicit semantic abstention causes may select a caller's fallback value.
#[derive(Clone, Debug)]
pub struct ValueFallback<T> {
    pub value: T,
    pub triggers: Vec<Reason>,
}
#[derive(Clone, Debug)]
pub struct PredicatePolicy {
    /// p_true <= reject_at accepts false; p_true >= accept_at accepts true.
    pub reject_at: f64,
    pub accept_at: f64,
    pub fallback: Option<ValueFallback<bool>>,
}
impl Default for PredicatePolicy {
    fn default() -> Self {
        Self {
            reject_at: 0.1,
            accept_at: 0.9,
            fallback: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ScorePolicy {
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
    pub min_confidence: Option<f64>,
    /// Absolute tolerance for normalization and expected-score consistency.
    pub epsilon: f64,
    pub fallback: Option<ValueFallback<f64>>,
}
impl Default for ScorePolicy {
    fn default() -> Self {
        Self {
            min_score: None,
            max_score: None,
            min_confidence: None,
            epsilon: 1e-6,
            fallback: None,
        }
    }
}
#[derive(Clone, Debug)]
pub enum ScalarPolicy {
    Predicate(PredicatePolicy),
    Score(ScorePolicy),
}
#[derive(Clone, Debug)]
pub enum ScalarPrediction {
    Predicate {
        question_id: String,
        probability_true: f64,
    },
    Score {
        question_id: String,
        score: f64,
        probabilities: Vec<(usize, f64)>,
        confidence: Confidence,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarValue {
    Predicate(bool),
    Score(f64),
}
#[derive(Clone, Debug)]
pub enum ScalarEvent {
    Prediction(ScalarPrediction),
    Refusal,
    Failed(BackendFailure),
    Cancelled,
}
#[derive(Clone, Copy, Debug)]
pub struct ScalarCapabilities {
    pub predicate: bool,
    pub score: bool,
    pub max_text_bytes: usize,
    pub max_score_levels: usize,
}
/// Additional backend boundary; existing Choice Backend implementations stay valid.
pub trait ScalarBackend {
    fn scalar_capabilities(&self) -> ScalarCapabilities;
    fn invoke_scalar(&mut self, request: &ScalarRequest) -> ScalarEvent;
}
pub(crate) struct ScalarContext<'a> {
    request: &'a ScalarRequest,
    policy: &'a ScalarPolicy,
    capabilities: ScalarCapabilities,
    prediction: Option<&'a ScalarPrediction>,
}
impl ScalarContext<'_> {
    fn confidence(&self) -> Option<&Confidence> {
        match self.prediction {
            Some(ScalarPrediction::Score { confidence, .. }) => Some(confidence),
            _ => None,
        }
    }
    fn max_score(&self) -> f64 {
        match self.request {
            ScalarRequest::Score(r) => r.levels.len().saturating_sub(1) as f64,
            _ => 0.0,
        }
    }
}
