use crate::numeric::sum;
use crate::scalar::{ScalarContext, ScalarPolicy, ScalarPrediction};
use specification_core::Specification;
pub(crate) struct ScalarValueConsistent;
impl Specification<ScalarContext<'_>> for ScalarValueConsistent {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match (c.prediction, c.policy) {
            (Some(ScalarPrediction::Predicate { .. }), ScalarPolicy::Predicate(_)) => true,
            (
                Some(ScalarPrediction::Score {
                    score,
                    probabilities,
                    ..
                }),
                ScalarPolicy::Score(p),
            ) => {
                let expected = sum(probabilities
                    .iter()
                    .map(|(level, v)| *level as f64 * v)
                    .collect());
                score.is_finite()
                    && (0.0..=c.max_score()).contains(score)
                    && (score - expected).abs() <= p.epsilon
            }
            _ => false,
        }
    }
}
