use crate::Confidence;
use crate::scalar::{ScalarContext, ScalarPolicy, ScalarPrediction};
use specification_core::Specification;
pub(crate) struct ScalarPolicySatisfied;
impl Specification<ScalarContext<'_>> for ScalarPolicySatisfied {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match (c.prediction, c.policy) {
            (
                Some(ScalarPrediction::Predicate {
                    probability_true: v,
                    ..
                }),
                ScalarPolicy::Predicate(p),
            ) => *v <= p.reject_at || *v >= p.accept_at,
            (
                Some(ScalarPrediction::Score {
                    score, confidence, ..
                }),
                ScalarPolicy::Score(p),
            ) => {
                p.min_score.is_none_or(|t| *score >= t)
                    && p.max_score.is_none_or(|t| *score <= t)
                    && p.min_confidence
                        .is_none_or(|t| matches!(confidence,Confidence::Present(v) if *v >= t))
            }
            _ => false,
        }
    }
}
